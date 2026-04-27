use borsh::{BorshDeserialize, BorshSerialize};
use clap::{Parser, Subcommand};
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{read_keypair_file, Signer},
    system_program,
    transaction::Transaction,
};
use std::str::FromStr;

const STATE_SEED: &[u8] = b"naive_character_state";

const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct NaiveCharacterState {
    pub is_initialized: bool,
    pub character_owner: Pubkey,
    pub matchmaking_tier: u8,
    pub permissions_level: u8,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum NaiveInstruction {
    Initialize {
        seller: Pubkey,
        matchmaking_tier: u8,
        permissions_level: u8,
    },
    Transfer {
        seller: Pubkey,
        new_owner: Pubkey,
    },
}

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(long)]
    url: String,

    #[arg(long)]
    program_id: String,

    #[arg(long)]
    payer: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Pda {
        #[arg(long)]
        seller: String,
    },
    Init {
        #[arg(long)]
        seller: String,
        #[arg(long)]
        tier: u8,
        #[arg(long)]
        permissions: u8,
    },
    Transfer {
        #[arg(long)]
        current_owner: String,
        #[arg(long)]
        seller: String,
        #[arg(long)]
        new_owner: String,
    },
    Read {
        #[arg(long)]
        seller: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let rpc = RpcClient::new(cli.url.clone());
    let payer = read_keypair_file(&cli.payer).expect("failed to read payer keypair");
    let program_id = Pubkey::from_str(&cli.program_id).expect("invalid program id");

    match cli.command {
        Commands::Pda { seller } => {
            let seller_pk = Pubkey::from_str(&seller).expect("invalid seller pubkey");
            let (state_pda, bump) =
                Pubkey::find_program_address(&[STATE_SEED, seller_pk.as_ref()], &program_id);

            println!("\n==============");
            println!("PDA DERIVATION");
            println!("seller: {}", seller_pk);
            println!("state_pda: {}", state_pda);
            println!("bump: {}", bump);
        }

        Commands::Init {
            seller,
            tier,
            permissions,
        } => {
            let seller_pk = Pubkey::from_str(&seller).expect("invalid seller pubkey");
            let (state_pda, _) =
                Pubkey::find_program_address(&[STATE_SEED, seller_pk.as_ref()], &program_id);

            println!("\n====================");
            println!("NAIVE INIT");
            println!("====================");
            println!("seller: {}", seller_pk);
            println!("matchmaking_tier: {}", tier);
            println!("permissions_level: {}", permissions);
            println!("state_pda: {}", state_pda);

            let ix = Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(state_pda, false),
                    AccountMeta::new(payer.pubkey(), true),
                    AccountMeta::new_readonly(system_program::ID, false),
                ],
                data: borsh::to_vec(&NaiveInstruction::Initialize {
                    seller: seller_pk,
                    matchmaking_tier: tier,
                    permissions_level: permissions,
                })
                .unwrap(),
            };

            let bh = rpc.get_latest_blockhash().unwrap();
            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&payer.pubkey()),
                &[&payer],
                bh,
            );

            let sig = rpc.send_and_confirm_transaction(&tx).unwrap();
            println!("NAIVE INIT OK");
            println!("signature: {}", sig);
        }

        Commands::Transfer {
            current_owner,
            seller,
            new_owner,
        } => {
            let current_owner_kp =
                read_keypair_file(&current_owner).expect("failed to read current_owner keypair");
            let seller_pk = Pubkey::from_str(&seller).expect("invalid seller pubkey");
            let new_owner_pk = Pubkey::from_str(&new_owner).expect("invalid new_owner pubkey");

            let (state_pda, _) =
                Pubkey::find_program_address(&[STATE_SEED, seller_pk.as_ref()], &program_id);

            println!("\n==============");
            println!("NAIVE TRANSFER");
            println!("seller: {}", seller_pk);
            println!("new_owner: {}", new_owner_pk);
            println!("state_pda: {}", state_pda);

            let ix = Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(state_pda, false),
                    AccountMeta::new_readonly(current_owner_kp.pubkey(), true),
                ],
                data: borsh::to_vec(&NaiveInstruction::Transfer {
                    seller: seller_pk,
                    new_owner: new_owner_pk,
                })
                .unwrap(),
            };

            let bh = rpc.get_latest_blockhash().unwrap();
            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&payer.pubkey()),
                &[&payer, &current_owner_kp],
                bh,
            );

            let sig = rpc.send_and_confirm_transaction(&tx).unwrap();
            println!("NAIVE TRANSFER OK — state not separated");
            println!("signature: {}", sig);
        }

        Commands::Read { seller } => {
            let seller_pk = Pubkey::from_str(&seller).expect("invalid seller pubkey");
            let (state_pda, _) =
                Pubkey::find_program_address(&[STATE_SEED, seller_pk.as_ref()], &program_id);

            let account = rpc.get_account(&state_pda).expect("failed to fetch account");
            let state =
                NaiveCharacterState::try_from_slice(&account.data).expect("failed to decode state");

            println!("\n=============================");
            println!("NAIVE TRANSFER — FAILURE MODE");
            println!("state_pda: {}", state_pda);
            println!("new_owner: {}", state.character_owner);
            println!(
                "matchmaking_tier: {}{}   <- from previous player{}",
                YELLOW, state.matchmaking_tier, RESET
            );
            println!(
                "permissions_level: {}{}   <- from previous player{}",
                YELLOW,
                match state.permissions_level {
                    0 => "none".to_string(),
                    1 => "basic".to_string(),
                    2 => "veteran".to_string(),
                    v => format!("unknown({})", v),
                },
                RESET
            );

            println!("\n{}WARNING: This state belonged to the previous player{}", RED, RESET);
            println!("{}WARNING: It moved with the asset{}", RED, RESET);
        }
    }
}
