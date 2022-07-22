mod xmodem;
mod hp_object;

use std::time::Duration;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use console::style;
use serialport;

// TODO: Command line flags should be in a more useful order than
// alphabetical

/// Transfer file to and from calculator.
#[derive(Parser)]
#[derive(Debug)]
struct Cli {
    /// Operation to execute on PATH
    #[clap(subcommand)]
    command: Commands,

    // No default_value_t needed to declare that the argument is
    // optional if the argument is of type Option
    /// Serial port to use for data transfer
    #[clap(short, long, value_parser)]
    port: Option<PathBuf>,

    /// Baud rate to use on port
    #[clap(short, long, value_parser)]
    #[clap(value_parser = clap::value_parser!(u32).range(1..))]
    baud: Option<u32>,

    /// Finish remote server after file transfer
    #[clap(short, long, action, default_value_t = false)]
    finish: bool,

}


// It should be noted that Kermit compatibility exists mainly for the
// 48S series. 
#[derive(Subcommand)]
#[derive(Debug)]
enum Commands {
    /// Send file to Kermit server
    Ksend {
	#[clap(parse(from_os_str))]
	path: std::path::PathBuf,
    },

    // Ah! Because Subcommands are each extendable, we can add an
    // option to communicate with a server to this.

    // The amount of future-proofing here is insane.
    /// Send file to XModem server
    Xsend {
	#[clap(parse(from_os_str))]
	path: std::path::PathBuf,

	/// Send to direct XRECV, not server (bypasses server operations and uses 128-byte XModem)
	#[clap(short, long, action, default_value_t = false)]
	direct: bool,
    },

    /// Get file from Kermit server
    Kget {
	#[clap(parse(from_os_str))]
	path: std::path::PathBuf,

	/// Overwrite pre-existing file on computer if necessary
	#[clap(short, long, action, default_value_t = false)]
	overwrite: bool,
    },

    /// Get file from XModem server
    Xget {
	#[clap(parse(from_os_str))]
	path: std::path::PathBuf,

	/// Overwrite pre-existing file on computer if necessary
	#[clap(short, long, action, default_value_t = false)]
	overwrite: bool,

	/// Get from direct XSEND, not server (bypasses server operations)
	#[clap(short, long, action, default_value_t = false)]
	direct: bool,
    },

    /// Run HP object info check on `path` instead of file transfer
    Info {
	#[clap(parse(from_os_str))]
	path: PathBuf,
    },
}


fn get_serial_port(cli_port: Option<PathBuf>, cli_baud: Option<u32>) -> Box<dyn serialport::SerialPort> {
    let discovered_ports = serialport::available_ports().expect("No ports found!");
    let final_port = {
	if cli_port == None {
	    if discovered_ports.len() == 0 {
		println!("no port specified, no port found!");
		std::process::exit(1);
	    } else {
		// use first port from discovered_ports
		// use .clone() to get copyable String (from https://stackoverflow.com/a/38305901)
		discovered_ports.get(0).unwrap().port_name.clone()
	    }
	} else {
	    std::string::String::from(cli_port.unwrap().to_str().unwrap())
	}
    };

    let final_baud = {
	if cli_baud == None {
	    9600 // assume 9600 because that's the default on the 48, and probably others
	} else {
	    cli_baud.unwrap()
	}
    };
    
    serialport::new(final_port, final_baud)
	.timeout(Duration::from_millis(4000))
	.open().expect("Failed to open port")

}
// The finish argument is to be ignored (and a message printed) if the
// direct flag is set. That is the only time---again, so simple.

fn main() {
    let cli = Cli::parse();

    // Dispatch operation
    match &cli.command {
	Commands::Xsend { direct, path } => {
	    let mut port = get_serial_port(cli.port, cli.baud);
	    println!("Xsend, direct = {:?}, path = {:?}", direct, path);
	    if *direct {
		if cli.finish {
		    println!("{}: {}{}{}",
			     style("warning").on_yellow().bold(),
			     "ignoring flag ", style("-f").green(),
			     " (finish) used in XModem direct mode");
		}
		xmodem::send_file_normal(&path.to_path_buf(), &mut port);
	    } else {
		// send file to server
		println!("send file to server");
		xmodem::send_file_conn4x(&path.to_path_buf(), &mut port);
	    }
	},

	Commands::Xget { direct, path, overwrite } => {
	    let mut port = get_serial_port(cli.port, cli.baud);
	    println!("Xget, path = {:?}, overwrite = {:?}", path, overwrite);
	    xmodem::get_file(path, &mut port, direct);
	    
	},

	Commands::Ksend { path } => {
	    //let mut port = get_serial_port(cli.port, cli.baud);
	    println!("Ksend, path = {:?}", path);
	},

	Commands::Kget { path, overwrite } => {
	    //let mut port = get_serial_port(cli.port, cli.baud);
	    println!("Kget, path = {:?}, overwrite = {:?}", path, overwrite);
	},

	Commands::Info { path } => {
	    println!("Info mode, path = {:?}", path);
	    hp_object::crc_and_output(path);
	},
    }
}
