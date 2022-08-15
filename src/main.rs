mod xmodem;
mod hp_object;
mod kermit;
mod helpers;

use std::time::Duration;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use console::style;
use serialport;

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

    /*/// Get file from Kermit server
    Kget {
	#[clap(parse(from_os_str))]
	path: std::path::PathBuf,

	/// Overwrite pre-existing file on computer if necessary
	#[clap(short, long, action, default_value_t = false)]
	overwrite: bool,
    },*/

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

    /// Run HP object info check on `path` instead of transferring file
    Info {
	#[clap(parse(from_os_str))]
	path: PathBuf,
    },
}


fn get_serial_port(cli_port: Option<PathBuf>, cli_baud: Option<u32>) -> Box<dyn serialport::SerialPort> {
    let discovered_ports = serialport::available_ports().expect("No ports found!");
    
    let mut usb_serial_ports: Vec<serialport::SerialPortInfo> = Vec::new();

    // Sort through the ports and find only USB serial
    // ports. Sometimes other ports are present, and it's quite
    // unlikely that they would be for the calculator
    for p in &discovered_ports {
	match p.port_type {
	    serialport::SerialPortType::UsbPort(..) => {
		usb_serial_ports.push(p.clone());
	    },
	    _ => {},
	}
    }
    
    //println!("discovered_ports is {:?}", discovered_ports);
    
    let final_port = {
	if cli_port == None {
	    if usb_serial_ports.len() == 0 {
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

    // This is not how I would normally write a match statement, but I
    // didn't want to deal with the return type in the Err arm.
    let port = serialport::new(final_port, final_baud)
	.timeout(Duration::from_millis(1500))
	.open();
    match port {
	// e.description is a string,
	Err(ref e) => crate::helpers::error_handler(format!("Error: failed to open port: {}", e.description)),
	_ => {},
    }
    return port.unwrap();

}
// The finish argument is to be ignored (and a message printed) if the
// direct flag is set. That is the only time---again, so simple
// compared to HPex.
fn main() {
    let cli = Cli::parse();

    // Dispatch operation
    match &cli.command {
	Commands::Xsend { direct, path } => {
	    let mut port = get_serial_port(cli.port, cli.baud);
	    //println!("Xsend, direct = {:?}, path = {:?}", direct, path);
	    // we actually use {:?} on the filename so that it displays in quotes
	    println!("Sending {:?} {} on {}...",
		     style(path.file_name().unwrap()).yellow().bright(),
		     match direct {
			 true => "via direct XModem",
			 false => "to XModem server",
		     },
		     style(port.name().unwrap()).green().bright());
	    if *direct {
		// send file directly to XRECV
		if cli.finish {
		    println!("{}: {}{}{}",
			     style("warning").yellow().bright(),
			     "ignoring flag ", style("-f").green(),
			     " (finish) used in XModem direct mode");
		}
		xmodem::send_file_normal(&path.to_path_buf(), &mut port);
	    } else {
		// send file to server
		xmodem::send_file_conn4x(&path.to_path_buf(), &mut port, &cli.finish);
	    }
	    println!("{}", style("Done!").green().bright());
	    // I like the way this newline and indent looks.
	    print!("File info:\n  ");
	    hp_object::crc_and_output(path);
	},

	Commands::Xget { direct, path, overwrite } => {
	    let mut port = get_serial_port(cli.port, cli.baud);
	    //println!("Xget, path = {:?}, overwrite = {:?}", path, overwrite);
	    // get the actual path that the transfer wrote to
	    let final_path = xmodem::get_file(path, &mut port, direct, overwrite, &cli.finish);
	    // "of" is not the right preposition to use here, but it
	    // makes it clear that we're talking about the file after
	    // processing, stored on the computer's drive.
	    print!("Info of received file:\n  ");
	    hp_object::crc_and_output(&final_path);
	},

	Commands::Ksend { path } => {
	    let mut port = get_serial_port(cli.port, cli.baud);
	    println!("Sending {:?} via Kermit on {}...",
		     style(path.file_name().unwrap()).yellow().bright(),
		     style(port.name().unwrap()).green().bright());
	    //println!("Ksend, path = {:?}, finish = {:?}", path, cli.finish);
	    kermit::send_file(path, &mut port, cli.finish);
	    print!("File info:\n  ");
	    hp_object::crc_and_output(path);
	},
	// I am not implementing kermit receive at the moment, but I
	// probably will later, because it's the only easy way to get
	// ASCII data.
	
	/*Commands::Kget { path, overwrite } => {
	    //let mut port = get_serial_port(cli.port, cli.baud);
	    println!("Kget, path = {:?}, overwrite = {:?}", path, overwrite);
	},*/

	Commands::Info { path } => {
	    //println!("Info mode, path = {:?}", path);
	    hp_object::crc_and_output(path);
	},
    }
}
