use std::env;
use std::io::{self, Write};

use braille::{BrailleChar, BrailleCharUnOrdered, BrailleCharGridVector};

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

    pub fn from_file(img: image::DynamicImage, w: u32, h: u32) -> Self {
        let img = img.resize_exact(w, h, FilterType::Triangle).to_rgb8();
        let (w, h) = img.dimensions();

        let mut buf = Self::new(w as usize, h as usize);

        let flat = img.as_flat_samples();
        let bytes = flat.samples;

        for y in 0..h as usize {
            for x in 0..w as usize {
                let i = (y * w as usize + x) * 3;
                let r = bytes[i] as f32;
                let g = bytes[i + 1] as f32;
                let b = bytes[i + 2] as f32;

                buf.set(x, y, Vec3::new(r, g, b));
            }
        }

        return buf;
    }
}

fn main() {
    let mut args = env::args().skip(1);

    let file = args.next().expect("Missing input argument");

    let mut width = 0;
    let mut height = 0;

    let mut verbose = false;
    let mut help = false;

    for arg in args {
        let arg = arg.to_lowercase();

        if let Some((a, b)) = arg.split_once('x') {
            match (a.parse::<u32>(), b.parse::<u32>()) {
                (Ok(w), Ok(h)) => {
                    width = w;
                    height = h;
                }
                _ => eprintln!("Error: '{}' is not valid [int]x[int] format", arg),
            }
        } else if let Ok(w) = arg.parse::<u32>() {
            width = w;
        } else if arg == "-h" || arg == "--help" {
            help = true;
        } else if arg == "-v" || arg == "--verbose" {
            verbose = true;
        } else {
            eprintln!("Error: '{}' is invalid", arg);
        }
    }

    let img = image::open(&file).expect("Missing file name");
    let (w, h) = img.dimensions();

    if height == 0 {
        if width == 0 {
            width = 128;
        }
        height = h * width / w;
    }

    let mut buffer = Buffer::from_file(img, width, height);

    if help {
        println!("Preview any image file");
        println!("Usage: preview [FILENAME] [OPTIONS] [SETTINGS]");
        println!("Options:");
        println!("    -h, --help                        Print help");
        println!("    -v, --verbose                     Use verbose output: display informations about the resizing");
        println!("Settings:");
        println!("    [int]x[int]                       Specify output dimensions");
        println!("    [int]                             Specify output width: the height gets scaled accordingly");

        return;
    }

    if verbose {
        println!("Resizing image from {} x {} to {} x {}", w, h, width, height);
    }

    let (w, h) = buffer.dimensions();

    let mut grid: BrailleCharGridVector<BrailleCharUnOrdered> = BrailleCharGridVector::new(w/2, h/4);

    let w_ = w/2*2;
    let h_ = h/4*4;

    for y in 0..h_ {
        for x in 0..w_ {
            let oldpixel = buffer.get(x, y).clamp(Vec3::ZERO, Vec3::splat(255.0));

            let (b, nl) = match oldpixel.element_sum() {
                0.0..381.0 => (false, 0.0),
                _ => (true, 255.0)
            };
            let newpixel = Vec3::splat(nl);

            grid.set_unchecked(x, y, b);

            let mut quant_error = oldpixel - newpixel;

            quant_error /= 8.0;

            let right = x + 1 < buffer.width;
            let right2 = x + 2 < buffer.width;
            let left = x > 0;
            let down = y + 1 < buffer.height;
            let down2 = y + 2 < buffer.height;

            if right {
                *buffer.get_mut(x+1, y) += quant_error;
                if right2 {
                    *buffer.get_mut(x+2, y) += quant_error;
                }
                if down {
                    *buffer.get_mut(x+1, y+1) += quant_error;
                }
            }
            if down {
                *buffer.get_mut(x, y+1) += quant_error;
                if left {
                    *buffer.get_mut(x-1, y+1) += quant_error;
                }
                if down2 {
                    *buffer.get_mut(x, y+2) += quant_error;
                }
            }
        }
    }

    let mut out = String::with_capacity(h/4 * (w/2 + 1));

    for row in grid.array.chunks_exact(w/2) {
        for char in row {
            out.push(char.char());
        }
        out.push('\n');
    }

    io::stdout().lock().write_all(out.as_bytes()).unwrap();
}

