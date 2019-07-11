
import {Server, AddressInfo} from "net";
import * as msgpack from "msgpack-lite";

export default async function server() {
    let server = new Server();
    await new Promise((resolve, reject) => {
        server.on("error", reject);
        server.listen({
            port: 0,
            host: "localhost"
        }, resolve);
    });
    let addr = server.address() as AddressInfo;
    console.log(`${addr.address}:${addr.port}`);

    server.on("connection", (socket) => {
        (async () => {
            socket.setNoDelay(true);
            await new Promise((resolve, reject) => {
                let decodeStream = msgpack.createDecodeStream();
                let encodeStream = msgpack.createEncodeStream();
                encodeStream.pipe(socket);

                let reading_length = true;
                socket.pipe(decodeStream).on("data", (payload) => {
                    console.log("received: ", payload);
                    if (reading_length) {
                    } else {
                        // let response = msgpack.encode({a: "what what"});
                        // console.log("sending: ", response);
                        // socket.write(msgpack.encode(response.byteLength));
                        // socket.write(response);
                        let res = msgpack.encode(payload);
                        socket.write(msgpack.encode(res.byteLength));
                        socket.write(res);
                    }
                    reading_length = !reading_length;
                });

                socket.on("close", resolve);
                socket.on("error", reject);
            });
        })().catch(e => {
            console.error(`Dropping client connection: ${e.stack}`);
            socket.destroy();
        });
    });

    await new Promise((resolve, reject) => {
        server.on("close", resolve);
        server.on("error", reject);
    });
}

