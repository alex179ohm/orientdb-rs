# orientdb-rs

[![Build Status](https://travis-ci.org/wolf4ood/orientdb-rs.svg?branch=master)](https://travis-ci.org/wolf4ood/orientdb-rs)
[![Codecov Status](https://codecov.io/gh/wolf4ood/orientdb-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/wolf4ood/orientdb-rs)
[![Crates.io](https://img.shields.io/crates/v/orientdb-client.svg)](https://crates.io/crates/orientdb-client)
[![Crates.io Download](https://img.shields.io/crates/d/orientdb-client.svg)](https://crates.io/crates/orientdb-client)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Docs](https://docs.rs/orientdb-client/badge.svg)](https://docs.rs/orientdb-client)
[![dependency status](https://deps.rs/repo/github/wolf4ood/orientdb-rs/status.svg)](https://deps.rs/repo/github/wolf4ood/orientdb-rs)
[![Codacy Badge](https://api.codacy.com/project/badge/Grade/e29ea9b1c90a47f797d259de04eb595b)](https://www.codacy.com/app/wolf4ood/orientdb-rs?utm_source=github.com&amp;utm_medium=referral&amp;utm_content=wolf4ood/orientdb-rs&amp;utm_campaign=Badge_Grade)



## OrientDB Client


A Rust Client for OrientDB.


### Installation


Install from [crates.io](https://crates.io/)

```toml
[dependencies]
orientdb_client = "*"
```


### Example


#### Basic Usage Synchronous



```rust

use orientdb_client::{OrientDB};

fn main() -> Result<(), Box<std::error::Error>> {
    let client = OrientDB::connect(("localhost",2424))?;

    let session = client.session("demodb","admin","admin")?;

    let results : Vec<_> = session.query("select from V where id = :param").named(&[("param", &1)]).run()?.collect();


    println!("{:?}", results);

    Ok(())
}
```

#### Basic Usage Asynchronous


```rust
use async_std::task::block_on;
use futures::StreamExt;
use orientdb_client::aio::OrientDB;
use orientdb_client::OrientResult;

fn main() -> OrientResult<()> {
    block_on(async {
        let client = OrientDB::connect(("localhost", 2424)).await?;

        let session = client.session("demodb", "admin", "admin").await?;

        let mut stream = session.query("select from V limit 10").run().await?;

        while let Some(item) = stream.next().await {
            println!("Record {:?}", item?);
        }

        Ok(())
    })
}
```

### Development


#### Compiling



```
git clone https://github.com/wolf4ood/orientdb-rs.git
cd orientdb-rs
cargo build
```


#### Running Tests



You can use docker-compose to start an instance for testing. Use the env variable `ORIENTDB_SERVER`
in order to specify the version of OrientDB

```
cd docker-compose
export ORIENTDB_SERVER=3.0.23
docker-compose up -d
cd ..
cargo test
```
