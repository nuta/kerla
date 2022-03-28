import { Server, Socket } from 'net';
import { resolve } from 'node:path';
import { Firecracker, VmCountLimitError } from './firecracker';
import { logger } from './logger';

const CONNECTION_TIMEOUT_MS = parseInt(process.env['CONNECTION_TIMEOUT_MS']) || (60 * 1000);

async function denyConnection(client: Socket, reason: string): Promise<void> {
    logger.info(`Deny client ${client.remoteAddress}:${client.remotePort} (${reason})`);
    client.write('SSH-SERVER: Sorry, we denied your connection: ');
    client.write(reason);
    client.write('\n');
}

export class SshProxy {
    firecracker: Firecracker;

    constructor(firecracker: Firecracker) {
        this.firecracker = firecracker;
    }

    async startListening(host: string, port: number): Promise<void> {
        return new Promise((resolve) => {
            const proxyServer = new Server(async (client) => {
                logger.info(`new client: ${client.remoteAddress}:${client.remotePort}`);
                client.setTimeout(CONNECTION_TIMEOUT_MS);

                const fromLocal = client.remoteAddress === "127.0.0.1" || client.remoteAddress === client.localAddress;
                let vm;
                try {
                    const startedAt = process.hrtime.bigint();
                    vm = await this.firecracker.createInstance(fromLocal);
                    const elapsed = (process.hrtime.bigint() - startedAt) / BigInt(Math.pow(10, 6));
                    logger.info(`${client.remoteAddress}:${client.remotePort}: Created a MicroVM in ${elapsed} ms`);
                } catch (e) {
                    if (e instanceof VmCountLimitError) {
                        await denyConnection(client, 'reached to the maximum count of running VMs, try again later :)');
                    } else {
                        logger.error('failed to create a microVM: ' + e.stack);
                        await denyConnection(client, 'failed to spawn a microVM, try again later');
                    }

                    client.destroy();
                    return;
                }

                const sshSocket = await vm.connect(22);

                sshSocket.on('error', (e) => {
                    logger.warn("error on a server-vm socket");
                    logger.warn(e);
                    sshSocket.destroy();
                    vm.kill();
                    client.destroy();
                });

                client.pipe(sshSocket).pipe(client);

                client.on('timeout', () => {
                    client.destroy();
                });

                client.on('error', (e) => {
                    console.warn(e);
                    client.destroy();
                });

                client.on('close', () => {
                    client.destroy();
                });

                client.on('end', () => {
                    vm.kill();
                });
            });

            proxyServer.listen(port, host, () => {
                logger.info(`listening on tcp://${host}:${port}`);
                resolve();
            });
        });
    }
}
