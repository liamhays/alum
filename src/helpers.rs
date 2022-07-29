use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};

pub fn get_file_contents(path: &PathBuf) -> Vec<u8> {
    // This gives a Vec<u8>.
    // from https://www.reddit.com/r/rust/comments/dekpl5/comment/f2wminn/
    let file_contents = match std::fs::read(path) {
	// we have to make the match arms, well, match, so we return a
	// new Vec, which should never actually get created
	Err(e) => { error_handler(format!("couldn't read {}: {}", path.display(), e)); Vec::new() },
	Ok(bytes) => bytes
    };
    return file_contents;
}

// TODO: this should probably use colorized output, take a prefix argument, etc.
pub fn error_handler(err: std::string::String) {
    eprintln!("{}", err);
    std::process::exit(1);
}

// from https://www.reddit.com/r/rust/comments/bk7v15/my_next_favourite_way_to_divide_integers_rounding/
pub fn div_up(a: usize, b: usize) -> usize {
    // We *know* that the hint is exact, this is thus precisely the amount of chunks of length `b` each
    (0..a).step_by(b).size_hint().0
}

pub fn get_progress_bar(len: u64, label: std::string::String) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(ProgressStyle::default_bar()
		 // spaces don't matter in this fromat string
		 
		 // wide_bar means expand to fill space, :2 means
		 // surround with 2 spaces (I think).
		 .template(format!("{{wide_bar}} {{pos:>2}}/{{len:2}} {label} ({{percent}}%)").as_str())
		 .progress_chars("##-"));
    return pb;
}


pub fn get_spinner(label: std::string::String) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_bar()
	    .template("{spinner:} {msg}")
	    // I like this spinner. It's reminiscent of systemd and very readable.
            .tick_strings(&[
		"[=     ]",
		"[==    ]",
		"[===   ]",
		"[ ===  ]",
		"[  === ]",
		"[   ===]",
		"[    ==]",
		"[     =]",
		"[    ==]",
		"[   ===]",
		"[  === ]",
		"[ ===  ]",
		"[===   ]",
		"[==    ]",
		"[=     ]",

            ]),
    );
    	
    pb.set_message(label);
    pb.enable_steady_tick(120); // in ms
    /*

    thread::sleep(Duration::from_secs(1));
    pb.finish_with_message("Receiving file 'REMOTE'...done!");
    println!("File info: ");
     */
    return pb;

}
