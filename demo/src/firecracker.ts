import { spawn } from 'child_process';
import { ChildProcess } from 'child_process';
import * as tempy from 'tempy';
import * as request from 'superagent';
import { Socket } from 'net';
import { logger } from './logger';
import { Readable } from 'stream';
import execa = require('execa');

export interface MicroVmArgs {
    apiUrl: string;
    process: ChildProcess;
    macAddress: string;
    ipAddress: string;
    vmId: number;
}

export class MicroVm {
    apiUrl: string;
    process: ChildProcess;
    macAddress: string;
    ipAddress: string;
    vmId: number;
    killed: boolean;
    socks: Socket[];

    constructor(args: MicroVmArgs) {
        this.apiUrl = args.apiUrl.replaceAll('/', '%2F');
        this.process = args.process;
        this.macAddress = args.macAddress;
        this.ipAddress = args.ipAddress;
        this.vmId = args.vmId;
        this.killed = false;
        this.socks = [];
    }

    async setBootSource(kernelImagePath: string): Promise<void> {
        await request.put(this.apiUrl + '/boot-source').send({
            kernel_image_path: kernelImagePath,
            boot_args: 'console=ttyS0 reboot=k panic=1 pci=off',
        });
    }

    async setMachineConfig(vcpuCount: number, memorySizeMb: number): Promise<void> {
        await request.put(this.apiUrl + '/machine-config').send({
            vcpu_count: vcpuCount,
            mem_size_mib: memorySizeMb,
            ht_enabled: false,
        });
    }

    async setupDisk(diskImagePath: string): Promise<void> {
        await request.put(this.apiUrl + '/drives/rootfs').send({
            drive_id: 'rootfs',
            path_on_host: diskImagePath,
            is_root_device: true,
            is_read_only: false,
        });
    }

    async triggerAction(action: 'InstanceStart'): Promise<void> {
        await request.put(this.apiUrl + '/actions').send({
            action_type: action,
        });
    }

    async connect(port: number): Promise<Socket> {
        const tryConnect = async (): Promise<Socket> => {
            const sock = new Socket();
            return new Promise((resolve, reject) => {
                sock.connect(port, this.ipAddress, () => {
                    resolve(sock);
                });

                sock.on('error', (e) => {
                    reject(e);
                })
            });
        };

        // Avoid TCP SYN retransmission due to its exponential backoff.
        await new Promise((resolve) => setTimeout(resolve, 200))

        let sock;
        const RETRIES_MAX = 9;
        for (let i = 0; i < RETRIES_MAX; i++) {
            try {
                sock = await tryConnect();
                this.socks.push(sock);
                return sock;
            } catch (e) {
                await new Promise((resolve) => setTimeout(resolve, 10 << i))
            }
        }

        throw new Error("failed to connect to microVM");
    }

    async kill() {
        if (this.killed) {
            return;
        }

        for (const sock of this.socks) {
            try { sock.end(); } catch {}
        }

        logger.debug(`killing a microVM (vm_id=${this.vmId}, pid=${this.process.pid})`);
        this.process.kill();
        this.killed = true;
    }
}

function readLines(stream: Readable, callback: (line: string) => void) {
    let buf = Buffer.alloc(0);
    stream.on('data', (data: Buffer) => {
        buf = Buffer.concat([buf, data]);
        while (true) {
            const newlineOffset = buf.indexOf('\n');
            if (newlineOffset == -1) {
                break;
            }

            const line = buf.slice(0, newlineOffset).toString('utf-8').replace(/[\r\n]/g, '');
            callback(line);

            buf = buf.slice(newlineOffset + 1);
        }
    });
}

export class VmCountLimitError extends Error { }

const FIRECRACKER_TIMEOUT_MS = parseInt(process.env['FIRECRACKER_TIMEOUT_MS']) || (60 * 1000);
const NUM_RUNNING_VMS_MAX = parseInt(process.env['NUM_RUNNING_VMS_MAX']) || 16;
const VCPUS_PER_VM = parseInt(process.env['VCPUS_PER_VM']) || 1;
const MEMORY_PER_VM = parseInt(process.env['MEMORY_PER_VM']) || 128;
const VM_ID_MIN = 10;
const VM_ID_MAX = Math.min(254, VM_ID_MIN + NUM_RUNNING_VMS_MAX);

let nextVmId = 10;
const vmIdsInUse = new Set();

function allocVmId(): number {
    for (let i = 0; i < NUM_RUNNING_VMS_MAX; i++) {
        const vmId = VM_ID_MIN + (nextVmId++ % NUM_RUNNING_VMS_MAX);
        if (!vmIdsInUse.has(vmId)) {
            return vmId;
        }
    }

    throw new Error('failed to allocate vm ID');
}

async function runChecked(argv0: string, argv: string[]): Promise<void> {
    await execa(argv0, argv, { stdio: 'inherit' });
}

export async function createNetworkInterfaces(): Promise<void> {
    for (let vmId = VM_ID_MIN; vmId < VM_ID_MAX; vmId++) {
        const guestMacAddress = 'aa:fc:11:' + vmId.toString(16).padStart(6, '0').match(/.{2}/g).join(':');
        const ipAddress = `192.168.122.${vmId}`;
        const xml = `<host mac='${guestMacAddress}' name='vm-${vmId}' ip='${ipAddress}' />`;
        try {
            await runChecked('virsh', ['net-update', 'default', 'add', 'ip-dhcp-host', xml, "--config"]);
        } catch (e) { }
    }

    try {
        await runChecked('virsh', ['net-destroy', 'default']);
    } catch (e) { }
    await runChecked('virsh', ['net-start', 'default']);

    await runChecked('ip', ['link', 'set', 'virbr0', 'type', 'bridge', 'stp_state', '0']);
    await runChecked('ethtool', ['--offload', 'virbr0', 'tx', 'off']);
}

export class Firecracker {
    kernelImagePath: string;
    numRunning: number;

    constructor(
        kernelImagePath: string,
    ) {
        this.kernelImagePath = kernelImagePath;
        this.numRunning = 0;
    }

    async createInstance(ignoreVmQuota: boolean): Promise<MicroVm> {
        if (!ignoreVmQuota && this.numRunning >= NUM_RUNNING_VMS_MAX) {
            throw new Error('too many running VMs');
        }

        const vmId = allocVmId();
        const hostDevName = 'veth' + vmId;
        const ipAddress = `192.168.122.${vmId}`;
        const macAddress = `aa:fc:00:` + vmId.toString(16).padStart(6, '0').match(/.{2}/g).join(':');
        const guestMacAddress = 'aa:fc:11:' + vmId.toString(16).padStart(6, '0').match(/.{2}/g).join(':');

        await runChecked('ip', ['tuntap', 'del', hostDevName, 'mode', 'tap']);
        await runChecked('ip', ['tuntap', 'add', hostDevName, 'mode', 'tap']);
        await runChecked('ip', ['link', 'set', hostDevName, 'address', macAddress]);
        await runChecked('ip', ['link', 'set', hostDevName, 'master', 'virbr0']);
        await runChecked('ip', ['link', 'set', hostDevName, 'up']);

        const configFile = tempy.writeSync(JSON.stringify({
            'boot-source': {
                kernel_image_path: this.kernelImagePath,
                boot_args: 'console=ttyS0 pci=off',
            },
            'drives': [],
            'network-interfaces': [
                {
                    iface_id: 'eth0',
                    guest_mac: guestMacAddress,
                    host_dev_name: hostDevName,
                },
            ],
            'machine-config': {
                vcpu_count: VCPUS_PER_VM,
                mem_size_mib: MEMORY_PER_VM,
                ht_enabled: false,
            },
        }));

        const socketPath = tempy.file({ extension: 'sock' });
        console.time('time-to-boot')
        const cp = spawn(
            'firecracker',
            [
                '--api-sock', socketPath, '--config-file', configFile, '--id', `${vmId}`,
                '--level', 'Debug', '--log-path', '/dev/stderr',
            ], {
            stdio: ['pipe', 'pipe', 'inherit'],
        }) as ChildProcess;

        const vm = new MicroVm({ apiUrl: `http+unix://${socketPath}`, vmId, process: cp, macAddress, ipAddress });
        this.numRunning++;
        vmIdsInUse.add(vmId);
        cp.on('exit', () => {
            this.numRunning--;
            vmIdsInUse.delete(vmId);
        });

        setTimeout(() => {
            logger.info(`[vm: ${vmId}] killing due to the timeout`);
            vm.kill();
        }, FIRECRACKER_TIMEOUT_MS);

        process.on('exit', () => {
            vm.kill();
        });

        let printedTime = false;
        readLines(cp.stdout, (line) => {
            if (!printedTime) {
                console.timeEnd('time-to-boot')
                printedTime = true
            }

            logger.info(`[vm: ${vmId}] ${line}`);
        });

        return vm;
    }
}
