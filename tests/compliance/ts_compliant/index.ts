
import server from "./server";
import client from "./client";

async function main() {
    let args: string[];
    if (/node$/.test(process.argv[0])) {
        args = process.argv.slice(2);
    } else {
        args = process.argv.slice(1);
    }

    let command = args[0];
    if (!command) {
        usage("no command");
    }
    switch (command) {
        case "client":
            let address = args[1];
            if (!address) {
                usage("no address");
            }
            await client(address);
            break;
        case "server":
            await server();
            break;
        default:
            usage(`unknown command ${command}`);
    }
}

function usage(err: string) {
    console.log(`Error: ${err}`);
    console.log(`Usage: `);
    console.log(`   ts_compliant server`);
    console.log(`   ts_compliant client ADDRESS`);
    process.exit(1);
}

main().catch(e => {
    console.error(`Uncaught rejection: ${e.stack}`);
});

