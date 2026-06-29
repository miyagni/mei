//! mei: the reference harness. Consumes mei-agent, mei-session and mei-provider.

fn main() {
    // Everything (sessions, config, credentials) lives under the global config
    // directory; the harness needs it before it can do anything useful.
    if let Err(e) = mei_config::config_dir() {
        eprintln!("mei: {e}");
        std::process::exit(1);
    }
    // TODO: open the session store and the credential store under this directory
    // as the harness grows.
}
