
## What lavish does

`lavish` lets you declare services, and implement/consume them
easily from a variety of language.

It is opinionated:

  * It has its own schema language & compiler (written in Rust)
  * It targets (for now) Rust, Go, and TypeScript
  * It comes with its own RPC runtime for Rust
  * It's designed with "bidirectional framed MessagePack-RPC over TCP" in mind
  
## Schemas

Schemas can define "functions", that take arguments and return results.

```lavish
server fn log(message: string)
server fn get_version() -> (major: int64, minor: int64, patch: int64)
server fn shutdown()
```

All input parameters and output parameters (results) are named.

Functions can be namespaced:

```lavish
namespace utils {
    server fn log()
    server fn get_version()
}

namespace system {
    server fn shutdown()
}
```

Functions can be implemented by the server or the client:

```lavish
namespace session {
    // try to log in. if password authentication is
    // enabled, @get_password is called.
    server fn login(username: string)

    client fn get_password(username: string) -> (password: string)

    server fn logout()
}
```

Built-in types are:

  * `uint32`, `uint64`, `int32`, `int64`: Integers
  * `float32`, `float64`: Floating-point numbers
  * `bool`: Booleans
  * `bytes`: A raw byte array
  * `timestamp`: A date+time

Custom types can be declared:

```lavish
enum LoginType {
    Anonymous = "anonymous",
    Password = "password",
} 

struct Session {
    login_type: LoginType,
    connected_at: timestamp,
}

namespace session {
    server fn login() -> (session: Session)
    // etc.
}
```

Types can be made optional with `Option<T>`:

```
// password can be None in Rust, nil in Go, undefined in TypeScript
server fn login(password: Option<string>)
```

Arrays are declared with `Array<T>`:

```
server fn login(ciphers: Array<Cipher>)
```

Maps are declared with `Map<K, V>`:

```
server fn login(options: Map<string, string>)
```

Third-party schemas can be imported:

```
import itchio from "github.com/itchio/go-itchio"

namespace fetch {
    server fn game(id: int64) -> (game: Option<itchio.Game>)
}
```

(More on the import mechanism later.)

## Workspaces

A workspace is a directory that contains a `lavish-rules` file.

A `lavish-rules` file is like a `Makefile` for lavish - it tells
it what to compile, and for which language.

The `lavish` command-line tool compiles schema files to Rust, Go,
TypeScript code.

Each workspace:

  * targets a single language (Rust, Go, TypeScript)
  * can build various services
    * ...which share imports

## Making a clock service

Let's say we're writing a simple Go service that returns
the current time.

Before running the lavish compiler, our repo looks like:

```
- go.mod
- main.go
- services/
  - clock.lavish
  - lavish-rules
```

`clock.lavish` contains:

```lavish
server fn current_time() -> (time: timestamp)
```

And `lavish-rules` contains:

```lavish
target go

build clock from "./clock.lavish"
```

The lavish compiler accepts the path to the workspace:

> lavish compile ./services

After running the lavish compiler, our repo will look like:

```
- go.mod
- main.go
- services/
  - clock.lavish
  - lavish-rules
  - clock/       <-- generated
    - clock.go   <-- generated
```

We can now implement the clock server, with something like:

```go
package main

import (
    "github.com/fasterthanlime/clock/services/clock"
    "time"
)

func Handler() clock.ServerHandler {
    var h clock.ServerHandler

    h.OnCurrentTime(func () (clock.CurrentTimeResults, error) {
        res := clock.CurrentTimeResults{
            time: time.Now(),
        }
        return res, nil
    })

    return h
}
```

Finally, we can add a `lavish-rules` file to the top-level, so that
we can later seemlessly import it from other projects:

```
export "./services/clock.lavish" as clock
```

## Consuming the clock service from Rust

Let's say we want to call our clock service from rust.

Our initial Rust repo will look like:

```
- Cargo.toml
- src
  - main.rs
  - services/
    - lavish-rules
```

Our `lavish-rules` file will look like:

```lavish
target rust

build clock from "github.com/fasterthanlime/clock"
```

Running the compiler with:

> lavish compile ./src/services

...will complain that `clock` is missing.

Running:

> lavish fetch ./src/services

Will populate the `lavish-vendor` folder:

```
- Cargo.toml
- src
  - main.rs
  - lavish-rules
  - lavish-vendor/  <-- new
    - clock.lavish  <-- new
```

Running compile again will generate rust code:

```
- Cargo.toml
- src
  - main.rs
  - lavish-rules
    - lavish-vendor/
    - clock.lavish
  - clock/          <-- new
    - mod.rs        <-- new
```

Now, the `clock` module can be imported from Rust and used
to consume the service, with something like:

```rust
use futures::executor;
use romio::tcp::TcpStream;
mod clock;

type Error = Box<dyn std::error::Error + 'static>;

async fn example(pool: executor::ThreadPool) -> Result<(), Error> {
    // establish TCP connection (could have a helper for that)
    let conn = TcpStream::connect("localhost:5959".parse()?).await?;

    // create peer via TCP conn, don't implement any functions
    // from our side, pass async runtime
    let client = clock::Client::new(conn, None, pool.clone())?;

    {
        let time = clock::current_time::call(&handle, ()).await?.time;
        println!("Server time: {:#?}", time);
    }

    // when all handles go out of scope, the connection is closed
}
```

## Consuming the clock service from TypeScript

Initial repo:

```
- src/
  - main.ts
  - services/
    - lavish-rules
```

Contents of `lavish-rules`:

```
target ts

build clock from "github.com/itchio"
```

> `lavish fetch src/services`

```
- src/
  - main.ts
  - services/
    - lavish-rules
    - lavish-vendor/  <-- new
      - clock.lavish  <-- new
```

> `lavish compile src/services`

```
- src/
  - main.ts
  - services/
    - lavish-rules
    - lavish-vendor/
      - clock.lavish
    - clock        <-- new
      - index.ts   <-- new
```

We can then use it, from `index.ts`:

```typescript
import clock from "./services/clock"

async function main() {
    let socket = new net.Socket();
    await new Promise((resolve, reject) => {
        socket.on("error", reject);
        socket.connect({ host: "localhost", port: 5959 }, resolve);
    });

    let client = new clock.Client(socket);
    console.log(`Server time: `, await client.getTime());
    socket.close();
}

main().catch((e) => {
    console.error(e);
    process.exit(1);
});
```
