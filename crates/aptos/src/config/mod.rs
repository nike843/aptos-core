// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliError, CliResult, CliTypedResult};
use crate::common::utils::{read_from_file, write_to_user_only_file};
use crate::genesis::git::{from_yaml, to_yaml};
use crate::Tool;
use async_trait::async_trait;
use clap::ArgEnum;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::{generate, Shell};
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::str::FromStr;

/// Tool for configuration of the CLI tool
///
#[derive(Parser)]
pub enum ConfigTool {
    Init(crate::common::init::InitTool),
    GenerateShellCompletions(GenerateShellCompletions),
    SetGlobalConfig(SetGlobalConfig),
}

impl ConfigTool {
    pub async fn execute(self) -> CliResult {
        match self {
            ConfigTool::Init(tool) => tool.execute_serialized_success().await,
            ConfigTool::GenerateShellCompletions(tool) => tool.execute_serialized_success().await,
            ConfigTool::SetGlobalConfig(tool) => tool.execute_serialized_success().await,
        }
    }
}

/// Generates shell completion files
///
/// First generate the completion file, then follow the shell specific directions on how
/// to install the completion file.
#[derive(Parser)]
pub struct GenerateShellCompletions {
    /// Shell to generate completions for one of [bash, elvish, powershell, zsh]
    #[clap(long)]
    shell: Shell,
    /// File to output shell completions to
    #[clap(long, parse(from_os_str))]
    output_file: PathBuf,
}

#[async_trait]
impl CliCommand<()> for GenerateShellCompletions {
    fn command_name(&self) -> &'static str {
        "GenerateShellCompletions"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let mut command = Tool::command();
        let mut file = std::fs::File::create(self.output_file.as_path())
            .map_err(|err| CliError::IO(self.output_file.display().to_string(), err))?;
        generate(self.shell, &mut command, "aptos".to_string(), &mut file);
        Ok(())
    }
}

/// Set global configuration settings
#[derive(Parser, Debug)]
pub struct SetGlobalConfig {
    /// A configuration for where to place and use the config
    ///
    /// Workspace allows for multiple configs based on location, where
    /// Global allows for one config for every part of the code
    #[clap(long)]
    config_type: Option<ConfigType>,
}

#[async_trait]
impl CliCommand<()> for SetGlobalConfig {
    fn command_name(&self) -> &'static str {
        "SetGlobalConfig"
    }

    async fn execute(self) -> CliTypedResult<()> {
        // Load the global config
        let mut config = GlobalConfig::load()?;

        // Enable all features that are actually listed
        if let Some(config_type) = self.config_type {
            config.config_type = config_type;
        }

        config.save()
    }
}

const GLOBAL_CONFIG_PATH: &str = "~/.aptos/global_config.yaml";

/// A global configuration for global settings related to a user
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GlobalConfig {
    /// Whether to be using Global or Workspace mode
    pub config_type: ConfigType,
}

impl GlobalConfig {
    pub fn load() -> CliTypedResult<Self> {
        // TODO: Ensure this path works on all platforms
        let path = PathBuf::from(GLOBAL_CONFIG_PATH);
        if path.exists() {
            from_yaml(&String::from_utf8(read_from_file(path.as_path())?)?)
        } else {
            // If we don't have a config, let's load the default
            Ok(GlobalConfig::default())
        }
    }

    /// Get the config location based on the type
    pub fn get_config_location(&self) -> PathBuf {
        match self.config_type {
            ConfigType::Global => PathBuf::from("~/.aptos"),
            ConfigType::Workspace => PathBuf::from(".aptos"),
        }
    }

    fn save(&self) -> CliTypedResult<()> {
        write_to_user_only_file(
            PathBuf::from(GLOBAL_CONFIG_PATH).as_path(),
            "Global Config",
            &to_yaml(&self)?.into_bytes(),
        )
    }
}

const GLOBAL: &str = "global";
const WORKSPACE: &str = "workspace";

/// A configuration for where to place and use the config
///
/// Workspace allows for multiple configs based on location, where
/// Global allows for one config for every part of the code
#[derive(Debug, Copy, Clone, Serialize, Deserialize, ArgEnum)]
pub enum ConfigType {
    /// Per system user configuration put in `<HOME>/.aptos`
    Global,
    /// Per directory configuration put in `<CURRENT_DIR>/.aptos`
    Workspace,
}

impl Default for ConfigType {
    fn default() -> Self {
        // TODO: When we version up, we can change this to global
        Self::Workspace
    }
}

impl std::fmt::Display for ConfigType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ConfigType::Global => GLOBAL,
            ConfigType::Workspace => WORKSPACE,
        })
    }
}

impl FromStr for ConfigType {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            GLOBAL => Ok(Self::Global),
            WORKSPACE => Ok(Self::Workspace),
            _ => Err(CliError::CommandArgumentError(
                "Invalid config type, must be one of [global, workspace]".to_string(),
            )),
        }
    }
}
