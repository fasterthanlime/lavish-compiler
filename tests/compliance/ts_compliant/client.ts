
import {Socket} from "net";

export default async function client(address: string) {
    let [host, portString] = address.split(":");
    let port = parseInt(portString, 10);
    if (!host || !port) {
        throw new Error(`invalid address ${address}`);
    }

    let socket = new Socket();
    await new Promise((resolve, reject) => {
        socket.on("error", reject);
        socket.connect({
            host, port
        }, resolve);
    });

    socket.destroy();
    console.log("bye now!");
}
