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

pub fn get_progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(ProgressStyle::default_bar()
		 // spaces don't matter in this fromat string
		 
		 // wide_bar means expand to fill space, :2 means
		 // surround with 2 spaces (I think).
		 .template(format!("{{wide_bar}} {{pos:>2}}/{{len:2}} packets ({{percent}}%)").as_str())
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

// Convert c (which is probably a Unicode character) to an HP 48
// single-byte character.
pub fn char_to_hp_char(c: char) -> u8 {
    if (c as u8) < 127 {
	return c as u8;
    }
    
    match c {
	// Shaded Block
	'▒' => 0x7f,
        '∡' => 0x80,
	// x with overbar
        ' ' => 0x81, // might need to fix?
        '▽' => 0x82, '√' => 0x83, '∫' => 0x84, 'Σ' => 0x85, '▶' => 0x86, 'π' => 0x87, '∂' => 0x88, '≤' => 0x89, '≥' => 0x8a,
        '≠' => 0x8b, '𝛼' => 0x8c, '→' => 0x8d, '←' => 0x8e, '↓' => 0x8f, '↑' => 0x90, 'γ' => 0x91, 'δ' => 0x92, 'ε' => 0x93,
        'η' => 0x94, 'θ' => 0x95, 'λ' => 0x96, 'ρ' => 0x97, 'σ' => 0x98, 'τ' => 0x99, 'ω' => 0x9a, 'Δ' => 0x9b, 'Π' => 0x9c,
        'Ω' => 0x9d,
	// Black Square
        '■' => 0x9e,
        '∞' => 0x9f,
	// non-breaking space (Latin-1 Supplement)
        ' ' => 0xa0,
        '¡' => 0xa1, '¢' => 0xa2, '£' => 0xa3, '¤' => 0xa4, // currency sign
        '¥' => 0xa5, '¦' => 0xa6, // Broken Bar, best matches HP 48 symbol
        '§' => 0xa7, '¨' => 0xa8, // Combining Diaeresis
        '©' => 0xa9, 'ª' => 0xaa, // Feminine Ordinal Indicator
        '«' => 0xab, '¬' => 0xac, // Not Sign
        '­' => 0xad, // Soft Hyphen
        '®' => 0xae, '¯' => 0xaf, // Macron
        '°' => 0xb0, '±' => 0xb1, '²' => 0xb2, '³' => 0xb3, '´' => 0xb4, // Acute Accent
        'µ' => 0xb5, '¶' => 0xb6, '·' => 0xb7, // Middle Dot
        '¸' => 0xb8, // Cedilla
        '¹' => 0xb9, 'º' => 0xba, // Masculine Ordinal Indicator
        '»' => 0xbb, '¼' => 0xbc, '½' => 0xbd, '¾' => 0xbe, '¿' => 0xbf,
	'À' => 0xc0, 'Á' => 0xc1, 'Â' => 0xc2, 'Ã' => 0xc3, 
        'Ä' => 0xc4, 'Å' => 0xc5, 'Æ' => 0xc6, 'Ç' => 0xc7, 'È' => 0xc8,
	'É' => 0xc9, 'Ê' => 0xca, 'Ë' => 0xcb, 'Ì' => 0xcc, 
        'Í' => 0xcd, 'Î' => 0xce, 'Ï' => 0xcf, 'Ð' => 0xd0, 'Ñ' => 0xd1,
	'Ò' => 0xd2, 'Ó' => 0xd3, 'Ô' => 0xd4, 'Õ' => 0xd5, 
        'Ö' => 0xd6, '×' => 0xd7, 'Ø' => 0xd8, 'Ù' => 0xd9, 'Ú' => 0xda,
	'Û' => 0xdb, 'Ü' => 0xdc, 'Ý' => 0xdd, 'Þ' => 0xde, 
        'ß' => 0xdf, 'à' => 0xe0, 'á' => 0xe1, 'â' => 0xe2, 'ã' => 0xe3,
	'ä' => 0xe4, 'å' => 0xe5, 'æ' => 0xe6, 'ç' => 0xe7, 
        'è' => 0xe8, 'é' => 0xe9, 'ê' => 0xea, 'ë' => 0xeb, 'ì' => 0xec,
	'í' => 0xed, 'î' => 0xee, 'ï' => 0xef, 'ð' => 0xf0, 
        'ñ' => 0xf1, 'ò' => 0xf2, 'ó' => 0xf3, 'ô' => 0xf4, 'õ' => 0xf5,
	'ö' => 0xf6, '÷' => 0xf7, 'ø' => 0xf8, 'ù' => 0xf9, 
        'ú' => 0xfa, 'û' => 0xfb, 'ü' => 0xfc, 'ý' => 0xfd, 'þ' => 0xfe, 'ÿ' => 0xff,
	_ => 0x00,
    }
}
	    


pub fn get_unique_path(path: PathBuf) -> PathBuf {
    let mut counter = 0;
    // We loop starting with the counter at 0, until we find a
    // file that doesn't exist. This is a bit of a hack,
    // because we convert path to a String and then make a
    // Path back from a modified string.
    loop {
	let new_string = match counter {
	    0 => String::from(path.to_str().unwrap()),
	    _ => format!("{}.{:?}", path.to_str().unwrap(), counter),
	};
	// we have to use a PathBuf because it's an owned type
	let new_path = PathBuf::from(&new_string);
	
	if !new_path.exists() {
	    break new_path;
	}
	
	counter += 1;
    }
}
