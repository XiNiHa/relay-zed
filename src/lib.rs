use std::{env, fs};

use zed_extension_api as zed;

const SERVER_PATH: &str = "node_modules/.bin/relay-compiler";
const PACKAGE_NAME: &str = "relay-compiler";

struct RelayZed;

impl RelayZed {
    fn server_exists(&self) -> bool {
        fs::metadata(SERVER_PATH).is_ok_and(|metadata| metadata.is_file())
    }

    fn server_script_path(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        path_to_relay: Option<String>,
    ) -> zed::Result<String> {
        if let Some(path) = path_to_relay {
            println!("You've manually specified 'pathToBinary'. We cannot confirm this version of the Relay Compiler is supported by this version of the extension. I hope you know what you're doing.");
            return Ok(path);
        }

        let server_exists = self.server_exists();
        if server_exists {
            return Ok(SERVER_PATH.to_string());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let version = zed::npm_package_latest_version(PACKAGE_NAME)?;

        if !server_exists
            || zed::npm_package_installed_version(PACKAGE_NAME)?.as_ref() != Some(&version)
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let result = zed::npm_install_package(PACKAGE_NAME, &version);
            match result {
                Ok(()) => {
                    if !self.server_exists() {
                        Err(format!(
                            "installed package '{PACKAGE_NAME}' did not contain expected path '{SERVER_PATH}'",
                        ))?;
                    }
                }
                Err(error) => {
                    if !self.server_exists() {
                        Err(error)?;
                    }
                }
            }
        }

        Ok(SERVER_PATH.to_string())
    }
}

impl zed::Extension for RelayZed {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let settings = Settings::from_lsp_settings(zed::settings::LspSettings::for_worktree(
            language_server_id.as_ref(),
            worktree,
        )?);
        let server_path = self.server_script_path(language_server_id, settings.path_to_relay)?;

        let args = vec![
            env::current_dir()
                .unwrap()
                .join(server_path)
                .to_string_lossy()
                .to_string(),
            "lsp".to_string(),
            format!("--output={}", settings.lsp_output_level),
        ];

        let working_directory = settings
            .root_directory
            .unwrap_or_else(|| worktree.root_path());

        Ok(zed::Command {
            command: "/bin/sh".to_string(),
            args: vec![
                "-c".to_string(),
                format!(
                    r#"cd {}; "{}" {}"#,
                    working_directory,
                    zed::node_binary_path()?,
                    args.into_iter()
                        .map(|arg| format!(r#""{}""#, arg))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
            ],
            env: Vec::new(),
        })
    }
}

struct Settings {
    lsp_output_level: String,
    root_directory: Option<String>,
    path_to_relay: Option<String>,
}

impl Settings {
    fn from_lsp_settings(settings: zed::settings::LspSettings) -> Self {
        Settings {
            lsp_output_level: settings
                .settings
                .as_ref()
                .and_then(|s| {
                    s.get("lspOutputLevel")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string())
                })
                .unwrap_or("quiet-with-errors".to_string()),
            root_directory: settings.settings.as_ref().and_then(|s| {
                s.get("rootDirectory")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string())
            }),
            path_to_relay: settings.settings.as_ref().and_then(|s| {
                s.get("pathToBinary")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string())
            }),
        }
    }
}

zed::register_extension!(RelayZed);
