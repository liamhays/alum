use std::path::PathBuf;

// TODO: Check lengths of vectors before reading
fn calc_crc(crc: u32, nibble: u8) -> u32 {
    return (crc >> 4) ^ (((crc ^ nibble as u32) & 0xFu32) * 0x1081u32);
}

enum LengthState {
    SizeNext,
    ASCICNext,
    DirNext,
    FindEndMarker,
    Fixed,
}

#[derive(Debug)]
pub struct ObjectInfo {
    pub romrev: char,
    pub crc: std::string::String,
    pub length: u32,
}
fn prolog_to_length(prolog: u32) -> Option<LengthState> {
    //        DOBINT  DOREAL  DOEREL  DOCMP   DOECMP  DOCHAR  DOROMP
    for i in [0x2911, 0x2933, 0x2955, 0x2977, 0x299d, 0x29bf, 0x29e2] {
	if prolog == i {
	    return Some(LengthState::Fixed);
	}
    }
    //        DOARRY DOLNKARRY DOCSTR  DOHSTR  DOGROB  DOLIB   DOBAK   DOEXT0  DOCODE
    for i in [0x29e8, 0x2a0a,  0x2a2c, 0x2a4e, 0x2b1e, 0x2b40, 0x2b62, 0x2b88, 0x2dcc] {
	if prolog == i {
	    return Some(LengthState::SizeNext);
	}
    }

    // Note that I'm not sure how you're supposed to get a tagged
    // object---saving one to a variable and transferring to a
    // computer didn't work for me.
    //        DOIDNT  DOLAM   DOTAG
    for i in [0x2e48, 0x2e6d, 0x2afc] {
	if prolog == i {
	    return Some(LengthState::ASCICNext);
	}
    }

    // TODO: fix these address names
    //        DOUNIT  prog    algebraic
    for i in [0x2ada, 0x2d9d, 0x2ab8] {
	if prolog == i {
	    return Some(LengthState::FindEndMarker);
	}
    }
    //           DORRP (directory, RAM/ROM pointer)
    if prolog == 0x2a96 {
	return Some(LengthState::DirNext);
    }

    return None;
}

// This does not need to have Option because prolog_to_length already checks for all these prologs.
fn prolog_to_fixed_length(prolog: u32) -> Option<u32> {
    println!("prolog to fixed length");
    // We subtract the length of the prolog from these because it's
    // added in later.
    match prolog {
	// DOBINT
	0x2911 => Some(10 - 5),
	// DOREAL
	0x2933 => Some(21 - 5),
	// DOEREL
	0x2955 => Some(26 - 5),
	// DOCMP
	0x2977 => Some(37 - 5),
	// DOECMP
	0x299d => Some(47 - 5),
	// DOCHAR
	0x29bf => Some(7 - 5),
	// DOROMP
	0x2e92 => Some(11 - 5),
	// should never happen
	_ => None,
    }
}
	    
fn read_size(nibs: &Vec<u8>) -> Option<u32> {
    // We have to go at least 10 nibbles in; if the object is less
    // than that then something is wrong.
    if nibs.len() < 10 {
	return None;
    }
    println!("read size");
    let mut length = 0u32;
    for i in (5..10).rev() {
	length <<= 4;
	length |= nibs[i] as u32;
    }
    println!("length is {:?}", length);
    // Must include prolog nibbles in this checksum
    return Some(length + 5u32);
}

fn get_prolog(nibs: &Vec<u8>) -> Option<u32> {
    if nibs.len() < 5 {
	return None;
    }
    
    let mut prolog = 0u32;
    for i in (0..5).rev() {
	prolog <<= 4;
	prolog |= nibs[i] as u32;
    }
    return Some(prolog);
}

fn calc_object_size(nibs: &Vec<u8>) -> Option<u32> {
    let prolog = match get_prolog(&nibs) {
	Some(pro) => pro,
	None => return None,
    };
    let object_length_type = prolog_to_length(prolog);
    if object_length_type.is_none() {
	// We didn't recognize the tagged object in the file
	return None;
    } else {
	let object_length = match object_length_type {
	    Some(LengthState::SizeNext) => read_size(&nibs),
	    Some(LengthState::ASCICNext) => read_ascic_size(&nibs),
	    Some(LengthState::DirNext) => read_dir_size(&nibs),
	    Some(LengthState::Fixed) => prolog_to_fixed_length(prolog),
	    Some(LengthState::FindEndMarker) => read_size_to_end_marker(&nibs),
	    None => return None,
	};
	return Some(5u32 + object_length.unwrap());
    }
}

fn read_ascic_size(nibs: &Vec<u8>) -> Option<u32> {
    println!("read ascic size");
    // ASCIC size is encoded as a byte (so up to 255 characters). We
    // then need to go get more size, by reading the object that
    // follows the ASCIC data.
    let ascic_len = (nibs[1] << 4) + nibs[0];
    let ascic_region_len = 2 + ascic_len * 2; // nibbles

    // slice then reconvert to Vec
    let inner_nibbles = nibs[ascic_region_len as usize..].to_vec();


    // TODO: don't use unwrap, use something else
    return Some(calc_object_size(&inner_nibbles).unwrap() + ascic_region_len as u32);
    
}

// TODO: variable names in this function suck
fn read_ascix_size(nibs: &Vec<u8>) -> Option<u32> {
    println!("read ascix size");
    // ASCIX consists of <1 byte length, ASCII data, same 1 byte
    // length>.

    let ascix_len = (nibs[1] << 4) + nibs[0];
    let ascix_region_len = 2 + (ascix_len*2) + 2;

    // slice then reconvert to Vec
    // Start at nibble 2 (first length), add ascii data len, then second length.
    // ascix_len is in bytes, because characters are bytes, so we multiply by 2 to get nibbles
    let inner_nibbles = nibs[ascix_region_len as usize..].to_vec();
    let inner_region = calc_object_size(&inner_nibbles).unwrap();

    return Some(inner_region + ascix_region_len as u32);
    
}

// TODO: why do we add 1 here?
fn read_size_to_end_marker(nibs: &Vec<u8>) -> Option<u32> {
    println!("read_size_to_end_marker, nibs is {:?}", nibs);
    let mut mem_addr = 0u32; // address in Saturn memory, 5 nibbles
    for (pos, i) in nibs.iter().enumerate() {
	mem_addr <<= 4;
	mem_addr |= *i as u32;
	mem_addr &= 0xfffffu32; // Saturn uses 20-bit address
	println!("{:#x}", mem_addr);
	if mem_addr == 0xb2130 { // object end marker, reversed (actually 0x312b)
	    println!("found end marker, exiting");
	    return Some(pos as u32 + 1);
	}
    }
    return None;
}

    
fn read_dir_size(nibs: &Vec<u8>) -> Option<u32> {
    println!("read_dir_size");
    // A directory consists of the prolog (5 nibbles), attached
    // libraries (3 nibbles), an offset number (5 nibbles), and
    // 0x00000 (5 nibbles) indicating the end of the directory. The
    // calculator then reads the directory from end to beginning,
    // looking for 0x00000. We simply have to jump to the first object
    // and iterate over every object we find.

    // 5 + 3 + 5 + 5 = 18 nibbles in
    let mut index = 18usize;
    //let new_nibs = Vec::from(&nibs[index..]);

    // At 18 nibbles in, the first object is defined with an ASCIX
    // name followed by the contents of the object.
    /*for i in &new_nibs {
	print!("{:#x}, ", i);
    }*/

    while index < nibs.len() - 18 {
	let ascix_size = read_ascix_size(&nibs[index..].to_vec());
	index += ascix_size.unwrap() as usize;
	index += 5; // 5 nibble offset value after each object
	println!("  ascix_size: {:?}", ascix_size);
    }

    // Subtract the value of two object offsets, because we skip past
    // the first one at the start, and the very last object in the
    // directory has no offset.
    
    // Directory objects don't include object counts, so this is
    // really the best way to do this.
    return Some(index as u32 - 5 - 5);
}

// A real number (and possibly other types) gives different checksums
// on the 48 and the 50, even though they are the same length on the
// calculator. The files themselves differ by one nibble at the
// end---I don't know why---but I think HP 49 checksums should not be
// printed, as they are very likely to be incorrect.

// Returns an Option enclosing the integer value of the
// checksum. Convert to a hex string yourself.
pub fn crc_file(path: &PathBuf) -> Option<ObjectInfo> {
    let file_contents = match std::fs::read(path) {
	Err(_) => panic!("couldn't read file"),
	Ok(bytes) => bytes,
    };

    let romrev_header = &file_contents[0..6];

    if romrev_header != b"HPHP48" {
	// We refuse to parse HP 49 objects because they are likely to
	// produce incorrect values.
	return None;
    }

    let romrev = *&file_contents[7] as char;

    // split file_contents into bytes each containing one nibble
    let mut nibbles: Vec<u8> = Vec::new();
    for byte in &file_contents[8..] {
	nibbles.push(byte & 0xfu8); // low nibble
	nibbles.push(byte >> 4); // high nibble
    }

    /*for byte in &file_contents[8..28] {
	print!("{:#x}, ", byte);
    }
    println!();
    for nib in &nibbles[0..30] {
	print!("{:#x}, ", nib);
    }*/

    let prolog = match get_prolog(&nibbles) {
	Some(pro) => pro,
	None => return None,
    };

    // TODO: maybe this variable should be usize?
    let object_length = match prolog_to_length(prolog) {
	Some(LengthState::SizeNext) => read_size(&nibbles),
	Some(LengthState::ASCICNext) => read_ascic_size(&nibbles),
	Some(LengthState::DirNext) => read_dir_size(&nibbles),
	Some(LengthState::Fixed) => prolog_to_fixed_length(prolog),
	Some(LengthState::FindEndMarker) => read_size_to_end_marker(&nibbles),
	None => return None,
    };

    // If the inner size functions also return None, exit.
    if object_length.is_none() {
	return None;
    }
    // The HP 48 will expand a file send to the computer into bytes,
    // so an object that is an odd number of nibbles (like a real,
    // which is 21 nibbles), will be expanded to 22 nibbles on the
    // computer. Therefore, the calculating program must act depending
    // on the prolog of the object.

    // We have the actual number of nibbles the object occupies in
    // object_length, so we can iterate from the start to that many
    // nibbles.
    let mut crc = 0u32;
    println!("object_length is {:?}", object_length);
    println!("{:?}", &nibbles[0..1000]);
    for nibble in &nibbles[0..object_length.unwrap() as usize] {
	crc = calc_crc(crc, *nibble);
    }

    println!("length of nibbles is {:?}", nibbles.len());
    println!("crc is {:?}", crc);
    let initial_str = format!("{:#x}", crc).to_uppercase();

    return Some(ObjectInfo {
	romrev: romrev,
	crc: format!("#{}h", &initial_str[2..]),
	length: object_length.unwrap(),
    });
}
