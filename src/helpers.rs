use std::path::PathBuf;

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
