use clap::{command, Parser};
use filesync_lib::{auth::CA, vpn::VPNConfig};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
enum Args {
    Issue {
        name: String,
        file_name: Option<String>,
        ip: Option<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args {
        Args::Issue {
            name,
            file_name,
            ip,
        } => {
            let file_name = file_name.unwrap_or_else(|| String::new());
            dotenvy::dotenv()?;
            let config = VPNConfig::from_env()?;
            let ca = CA::load_or_create_with_config(&config)?;
            let (cert, key) =
                ca.issue_as_string(&name, ip.map(|ip| ip.parse().ok()).flatten(), false)?;
            CA::save_cert_and_key(&config, cert, key, &file_name)?;
        }
    }
    Ok(())
}
