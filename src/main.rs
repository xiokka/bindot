use rustic_bitmap::*;
use std::fs::File;
use std::env;
use std::io::Read;
use std::io;
use std::io::Write;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 3 {
        eprintln!("Invalid arguments. Usage: bindot [-i|-e] [image] [data]");
        return Ok(());
    }
    let operation = &args[1];
    let image = &args[2];
    let data = &args[3];

    if operation != "-i" && operation != "-e" {
        eprintln!("Invalid arguments. Usage: bindot [-i|-e] [image] [data]");
        return Ok(());
    }

    let mut file_image = File::open(image)?;
    let mut image_data = Vec::new();
    file_image.read_to_end(&mut image_data)?;
    if !image_data.has_file_signature() {
	eprintln!("{} is not a bitmap. Exiting.", image);
	eprintln!("If you have ffmpeg installed, you can easily convert it to a bitmap using the following command:");
	eprintln!("ffmpeg -i {} output.bmp", image);
	return Ok(());
    }

    if operation == "-i" {
	let mut binary_data_file = File::open(data)?;
        let mut binary_data = Vec::new();
        binary_data_file.read_to_end(&mut binary_data)?;
	println!("Original binary file size: {} bytes", binary_data.len());
	// Zero-pad the binary data to make its length divisible by 3 (makes it easier to divide into 3-byte triplets later on just trust me)
	let padding_len = (3 - (binary_data.len() % 3)) % 3;  // Calculate how many bytes to pad
	binary_data.extend(vec![0; padding_len]);  // Pad with zeros
        println!("Zero-padded binary file size: {} bytes", binary_data.len());

	// this for loop turns the binary data into a 1-pixel-tall bitmap
	let height = 1;
	let width: usize = binary_data.len() / 3;
	let mut bmp:Vec<u8> = Vec::<u8>::new_bitmap(width.try_into().unwrap(), height.try_into().unwrap(), 24);
	for j in 0..width {
		let walk = j*3;
		if walk + 2 < binary_data.len() {
			let color = Rgb {r: binary_data[walk + 2], g: binary_data[walk + 1], b: binary_data[walk]};
               		let position: Point = Point {x: j as u32, y: 0 as u32};
			bmp.draw_point(&position, &color);
		}
	}

	// Calculate step length between each slice
	let step_length = (image_data.get_width() * image_data.get_height() - 2) / bmp.get_width();
	println!("Step length: {}", step_length);
	if step_length < 1 {
		println!("File won't fit.");
		return Ok(());
	}

	// Insert number of pixels with hidden data into the first 4 bytes of the pixel array
	let len_bytes: Vec<u8> = bmp.get_width().to_le_bytes().to_vec();
	let offset = image_data.get_pixel_array_offset();
	for i in 0..len_bytes.len() {
		image_data[offset+i] = len_bytes[i];
	}

	// Insert hidden data
        let mut walk = Point {x: 2, y : 0};
        for i in 0..bmp.get_width() {
                let pixel_data = bmp.get_pixel(&Point{x: i, y:0}).unwrap();
                image_data.draw_point(&walk, &pixel_data);
		let next_x = walk.x + step_length;
		if next_x >= image_data.get_width() {
		    let overflow = next_x - image_data.get_width();
		    let width = image_data.get_width();
		    walk.x = overflow % width;
		    walk.y += 1 + (overflow / width);
		} else {
		    walk.x = next_x;
		}
	}

	let output_path = "bindot_output.bmp";
	let mut file_output = File::create(output_path).unwrap();
	file_output.write_all(&image_data).unwrap();
	println!("Image written to {}", output_path);
    }

    if operation == "-e" {
	// Get number of pixels with hidden data
	let offset = image_data.get_pixel_array_offset();
	let bytes = [
	    image_data[offset],
	    image_data[offset + 1],
	    image_data[offset + 2],
	    image_data[offset + 3],
	];
	let num_pixels = u32::from_le_bytes(bytes);
	let step_length = (image_data.get_width() * image_data.get_height() - 2) / num_pixels;

	let mut hidden_data = vec![];
        let mut walk = Point {x: 2, y : 0};
        for i in 0..num_pixels {
		let pixel = image_data.get_pixel(&walk).unwrap();
		hidden_data.push(pixel.b);
		hidden_data.push(pixel.g);
		hidden_data.push(pixel.r);
                let next_x = walk.x + step_length;
                if next_x >= image_data.get_width() {
                    let overflow = next_x - image_data.get_width();
                    let width = image_data.get_width();
                    walk.x = overflow % width;
                    walk.y += 1 + (overflow / width);
                } else {
                    walk.x = next_x;
                }
        }
	let mut file_output = File::create(data).unwrap();
	file_output.write_all(&hidden_data).unwrap();
	println!("Data from {} extracted to {}", image, data);
    }
    return Ok(());
}
