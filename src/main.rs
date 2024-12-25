mod binutils;
mod bot;
mod botlib;
mod cmds;
mod rpc;

pub use botlib::{Command, Context, Error};

#[tokio::main]
async fn main() {
    cmds::cmd_loader().await;
}
