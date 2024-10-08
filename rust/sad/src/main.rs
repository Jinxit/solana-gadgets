//! @brief Main entry poiint for CLI

use std::rc::Rc;

use clparse::get_account_and_descriptor;

use {
    desertree::Deseriaizer,
    sadout::{SadJsonOutput, SadOutput, SadSysOutput},
    solana_clap_utils::{input_validators::normalize_to_url_if_moniker, keypair::DefaultSigner},
    solana_client::rpc_client::RpcClient,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{commitment_config::CommitmentConfig, signature::Signer},
    std::process::exit,
};

/// sad main module
mod clparse;
mod desertree;
mod errors;
mod sadout;
mod sadtypes;
mod solq;

#[allow(dead_code)]
struct Config {
    commitment_config: CommitmentConfig,
    default_signer: Box<dyn Signer>,
    json_rpc_url: String,
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = clparse::parse_command_line();
    let app_matches = app.get_matches();
    let (sub_command, sub_matches) = app_matches.subcommand();
    let matches = sub_matches.unwrap_or_else(|| {
        clparse::parse_command_line().print_help().unwrap();
        println!("");
        exit(-1);
    });
    let mut wallet_manager: Option<Rc<RemoteWalletManager>> = None;
    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        let default_signer =
            DefaultSigner::new("keypair".to_string(), cli_config.keypair_path.clone());

        Config {
            json_rpc_url: normalize_to_url_if_moniker(
                matches
                    .value_of("json_rpc_url")
                    .unwrap_or(&cli_config.json_rpc_url)
                    .to_string(),
            ),
            default_signer: default_signer
                .signer_from_path(matches, &mut wallet_manager)
                .unwrap_or_else(|err| {
                    eprintln!("error: {}", err);
                    exit(1);
                }),
            verbose: matches.is_present("verbose"),
            commitment_config: CommitmentConfig::confirmed(),
        }
    };
    // Change to "solana=debug" if needed
    solana_logger::setup_with_default("solana=info");

    if config.verbose {
        println!("JSON RPC URL: {}", config.json_rpc_url);
    }
    let rpc_client = RpcClient::new(config.json_rpc_url.clone());

    // Arguments specific to deserialization

    // Setup the account or program public key
    let (target_pubkey, indecl) = get_account_and_descriptor(matches);
    // let target_pubkey = get_target_publickey(matches);

    // // Get the deserialization descriptor
    // let descriptor_file_name = matches.value_of("decl").unwrap();
    // let indecl = load_yaml_file(descriptor_file_name).unwrap_or_else(|err| {
    //     eprintln!("File error: On {} {}", descriptor_file_name, err);
    //     exit(1);
    // });

    // Setup the deserialization tree
    let destree = Deseriaizer::new(&indecl[0]);

    // Get deserialization results
    let deserialize_result = match sub_command {
        "account" => solq::deserialize_account(&rpc_client, &target_pubkey, &destree)?,
        "program" => solq::deserialize_program_accounts(&rpc_client, &target_pubkey, &destree)?,
        _ => unreachable!(),
    };
    // Check for output or default to pretty print
    match matches.value_of("output").unwrap() {
        "json" => SadJsonOutput::new(
            deserialize_result,
            destree,
            matches.value_of("filename").unwrap(),
        )
        .write()?,
        "stdout" => SadSysOutput::new(deserialize_result, destree).write()?,
        _ => unreachable!(),
    };
    Ok(())
}
