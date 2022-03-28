import { logger } from './logger';
import { SshProxy } from './ssh_proxy';
import { createNetworkInterfaces, Firecracker } from './firecracker';
import { fork, spawnSync, } from 'node:child_process';
import { Socket } from 'node:net';

const APP_HOST = process.env['APP_HOST'] ?? '127.0.0.1';
const APP_PORT = parseInt(process.env['APP_PORT']) || 7272;
const HEALTH_CHECK_INTERVAL_MS = parseInt(process.env['HEALTH_CHECK_INTERVAL_MS']) || 10 * 1000;
const KERNEL_IMAGE_PATH = process.env['KERNEL_IMAGE_PATH'] ?? `${process.env.HOME}/boot.elf`;

async function serverMain() {
    logger.info('launch a server...');
    const firecracker = new Firecracker(KERNEL_IMAGE_PATH);
    const sshProxy = new SshProxy(firecracker);
    await sshProxy.startListening(APP_HOST, APP_PORT);

    setInterval(() => {
        if (firecracker.numRunning > 0) {
            logger.info(`running ${firecracker.numRunning} microVMs`)
        }
    }, 60 * 1000);

    if (process.send) {
        process.send("launched");
    }
}

function healthCheck(): Promise<void> {
    return new Promise((resolve, reject) => {
        let dead = true;
        const sock = new Socket();
        sock.connect(APP_PORT, "localhost");
        sock.setTimeout(HEALTH_CHECK_INTERVAL_MS / 2);

        sock.on('connect', () => {
            sock.write("SSH-2.0-healthcheck HI \r\n");
        });

        let buf = Buffer.alloc(0);
        sock.on('data', (data) => {
            buf = Buffer.concat([buf, data]);
            if (buf.indexOf("SSH-2.0-dropbear") >= 0) {
                dead = false;
                sock.end();
                resolve();
            }
        });

        sock.on('error', reject);
        sock.on('end', () => {
            if (dead) {
                reject(new Error("received 'end' event on the health check event with dead == true"));
            }
        });
    });
}

function crashloop(): Promise<void> {
    return new Promise((_resolve, rejectRaw) => {
        const cp = fork(__filename, { env: Object.assign({ SERVER_MAIN: "1" }, process.env) });

        let launched = false;
        let killed = false;
        let healthCheckTimer = null;

        const reject = (e) => {
            if (healthCheckTimer) {
                clearInterval(healthCheckTimer);
            }

            rejectRaw(e);
        }

        const activateHealthCheck = () => {
            healthCheckTimer = setInterval(() => {
                console.time('time-to-first-byte')
                healthCheck()
                    .catch(e => {
                        cp.kill();
                        killed = true;
                        reject(e);
                    }).finally(() =>{
                        console.timeEnd('time-to-first-byte')
                    });
            }, HEALTH_CHECK_INTERVAL_MS);
        };

        cp.on('message', message => {
            switch (message) {
                case "launched":
                    launched = true;
                    activateHealthCheck();
                    logger.info("confirmed the successful launch, health check activated");
                    break;
            }
        })

        setTimeout(() => {
            if (!launched) {
                reject(new Error("the server process failed to initialize itself"));
            }
        }, 5000);

        cp.on('exit', (code, _signal) => {
            if (!killed) {
                reject(new Error(`process has crashed with ${code}`));
            }
        });
    });
}

async function main() {
    logger.info('initializing the virtual network');
    await createNetworkInterfaces();

    logger.info('in the crashloop');
    while (true) {
        logger.info("starting the server...");
        spawnSync("sudo", ["killall", "firecracker"]);

        const [start] = process.hrtime();
        try {
            await crashloop();
        } catch (e) {
            logger.warn("server has crashed:");
            logger.warn(e);
        } finally {
            const [end] = process.hrtime();
            if (end - start < 5) {
                logger.error("too early crash, sleeping 10 seconds...");
                await new Promise((resolve) => setTimeout(resolve, 10 * 1000));
            }
        }
    }
}

if (process.env.SERVER_MAIN) {
    serverMain();
} else {
    main();
}
