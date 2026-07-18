use anyhow::Result;

use crate::api::client::ApiClient;
use crate::cli::{Cli, Cmd, GlobalArgs};
use crate::config::Config;

mod elementdata;
mod elements;
mod folders;
mod layouts;
mod mcp;
mod osc;
mod overlay;
mod ping;
mod version;
mod whoami;

pub fn run(cli: Cli) -> Result<()> {
    let global = cli.global;
    match cli.cmd {
        Cmd::Ping => ping::run(&global),
        Cmd::Whoami => whoami::run(&global),
        Cmd::Overlay(c) => overlay::run(c.cmd, &global),
        Cmd::Elements(c) => elements::run(c.cmd, &global),
        Cmd::Elementdata(c) => elementdata::run(c.cmd, &global),
        Cmd::Layouts(c) => layouts::run(c.cmd, &global),
        Cmd::Folders(c) => folders::run(c.cmd, &global),
        Cmd::Version(c) => version::run(c),
        Cmd::Mcp => mcp::run(&global),
        Cmd::Osc(c) => osc::run(c, &global),
    }
}

pub fn client_for(global: &GlobalArgs) -> Result<ApiClient> {
    let config = Config::load()?;
    let profile = config.resolve(global.overlay.as_deref())?;
    Ok(ApiClient::new(&profile.address, &profile.token))
}
