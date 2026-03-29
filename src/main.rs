use std::env;

use braille::{BrailleCharUnOrdered, BrailleCharGridVector};

use glam::Vec3;
use image::{imageops::FilterType, GenericImageView};

struct Buffer {
    pub width: usize,
    pub height: usize,
    pub data: Vec<Vec3>,
}

impl Buffer {
    pub fn new(width: usize, height: usize) -> Self {
        return Self {
            width,
            height,
            data: vec![Vec3::ZERO; width * height]
        };
    }

    pub fn dimensions(&self) -> (usize, usize) {
        return (self.width, self.height);
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> Vec3 {
        return self.data[y * self.width + x];
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, value: Vec3) {
        self.data[y * self.width + x] = value;
    }

    #[inline]
    fn get_mut(&mut self, x: usize, y: usize) -> &mut Vec3 {
        return &mut self.data[y * self.width + x];
    }

    pub fn from_file(img: image::DynamicImage, w: u32, h: u32, filter: FilterType) -> Self {
        let img = img.resize_exact(w, h, filter).to_rgb8();
        let (w, h) = img.dimensions();

        let mut buf = Self::new(w as usize, h as usize);

        for y in 0..h as usize {
            for x in 0..w as usize {
                let p = img.get_pixel(x as u32, y as u32).0;
                buf.set(x, y, Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32));
            }
        }

        return buf;
    }
}

fn dither_img(mut img: Buffer) {
    let (w, h) = img.dimensions();

    let mut array: BrailleCharGridVector<BrailleCharUnOrdered> = BrailleCharGridVector::new(w/2, h/4);

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get(x, y);
            let l = pixel.element_sum() /  3.0;
            img.set(x, y, Vec3::splat(l));
        }
    }

    for y in 0..(h/4*4) {
        for x in 0..(w/2*2) {
            let oldpixel = img.get(x, y).clamp(Vec3::ZERO, Vec3::splat(255.0));

            let (b, nl) = match oldpixel.x {
                0.0..127.0 => (false, 0.0),
                _ => (true, 255.0)
            };
            let newpixel = Vec3::splat(nl);

            array.set(x, y, b);

            let quant_error = oldpixel - newpixel;

            let right = x + 1 < w;
            let down  = y + 1 < h;
            let left  = x > 0;

            if right {
                *img.get_mut(x+1, y) += quant_error * 7.0/ 16.0;
            }
            if down {
                if left {
                    *img.get_mut(x-1, y+1) += quant_error * 3.0/ 16.0;
                }
                *img.get_mut(x, y+1) += quant_error * 5.0 / 16.0;
                if right {
                    *img.get_mut(x+1, y+1) += quant_error * 1.0/ 16.0;
                }
            }
        }
    }

    for y in 0..(h/4) {
        for x in 0..(w/2) {
            print!("{}", array.get_char_unchecked(x, y).char());
        }
        println!();
    }
}

fn main() {
    let mut args = env::args().skip(1);

    let file = args.next().expect("Missing file name");

    let h = file.to_lowercase();
    if h == "-h" || h == "--help" {
        println!("Preview any image file");
        println!("Usage: preview [FILENAME] [OPTIONS] [SETTINGS]");
        println!("Options:");
        println!("    -h, --help                        Print help");
        println!("    -v, --verbose                     Use verbose output: display informations about the resizing");
        println!("Settings:");
        println!("    [int]x[int]                       Specify output dimensions");
        println!("    [filtertype]                      Specify filter type for resizing:");
        println!("        'n' | 'nearest' | 'near'          Nearest");
        println!("        't' | 'triangle'                  Triangle");
        println!("        'c' | 'catmullrom'                CatmullRom");
        println!("        'g' | 'gaussian' | 'gauss'        Gaussian");
        println!("        'l' | 'lanczos3' | 'lanczos'      Lanczos3");

        return;
    }

    let mut filter = FilterType::Nearest;
    let mut width = 0;
    let mut height = 0;

    let mut seen_filter = false;
    let mut seen_dims = false;
    let mut verbose = false;

    for arg in args.take(3) {
        let arg = arg.to_lowercase();

        if let Some((a, b)) = arg.split_once('x') {
            if seen_dims {
                eprintln!("Error: dimensions already provided");
                continue;
            }

            match (a.parse::<u32>(), b.parse::<u32>()) {
                (Ok(w), Ok(h)) => {
                    width = w;
                    height = h;
                    seen_dims = true;
                }
                _ => eprintln!("Error: '{}' is not valid [int]x[int] format", arg),
            }
        }
        else if arg == "-v" || arg == "--verbose" {
            verbose = true;
        }
        else if arg == "nearest" || arg == "near" || arg == "n" {
            if seen_filter {
                eprintln!("Error: filter type already set");
            } else {
                filter = FilterType::Nearest;
                seen_filter = true;
            }
        }
        else if arg == "triangle" || arg == "t" {
            if seen_filter {
                eprintln!("Error: filter type already set");
            } else {
                filter = FilterType::Triangle;
                seen_filter = true;
            }
        }
        else if arg == "catmullrom" || arg == "c" {
            if seen_filter {
                eprintln!("Error: filter type already set");
            } else {
                filter = FilterType::CatmullRom;
                seen_filter = true;
            }
        }
        else if arg == "gaussian" || arg == "gauss" || arg == "g" {
            if seen_filter {
                eprintln!("Error: filter type already set");
            } else {
                filter = FilterType::Gaussian;
                seen_filter = true;
            }
        }
        else if arg == "lanczos3" || arg == "lanczos" || arg == "l" {
            if seen_filter {
                eprintln!("Error: filter type already set");
            } else {
                filter = FilterType::Lanczos3;
                seen_filter = true;
            }
        }

        else {
            eprintln!("Error: '{}' is invalid", arg);
        }
    }

    let img = image::open(&file).expect("Missing file name");
    let (w, h) = img.dimensions();

    if width == 0 || height == 0 {
        width = 128;
        height = h * 128 / w;
    }

    let buffer = Buffer::from_file(img, width, height, filter);

    if verbose {
        println!("Resizing image from {} x {} to {} x {} using {:?} filtering", w, h, width, height, filter);
    }

    dither_img(buffer);
}

