use ficture::cell::Cell;
use ficture::config::Config;
use ficture::image::pixel_map_to_image;
use ficture::map::{Map, MapMonad};
use ficture::noise::SimplexNoiseGeneratorBuilder;
use ficture::utils::normalize;

mod args;

use args::{Args, Parser};

fn main() {
    let args = Args::parse();
    let config = Config::from_file(args.filepath).expect("config filename to be provided");

    let elevation_noise_generator = config
        .get_noise_generator::<SimplexNoiseGeneratorBuilder>("elevation_noise", args.width, args.height)
        .expect("elevation noise to be defined");
    let moisture_noise_generator = config
        .get_noise_generator::<SimplexNoiseGeneratorBuilder>("moisture_noise", args.width, args.height)
        .expect("moisture noise to be defined");
    let evaluator = config
        .get_color_evaluator("default")
        .expect("default biomes to be define");
    let ocean = config.get_color_func("ocean").expect("ocean to be defined");
    let sea_level = 0.05;

    let map: Map<Cell> = Map::return_single(
        Cell {
            elevation: 0.0,
            moisture: 0.0,
        },
        args.width,
        args.height,
    );

    // use noise to create the heightmap and moisture map
    let map = map.and_then_with_coordinates(|_, x, y| Cell {
        elevation: elevation_noise_generator.generate(x, y),
        moisture: moisture_noise_generator.generate(x, y),
    });

    // get min and max moisture for use in normalization
    let (min_elevation, max_elevation, min_moisture, max_moisture) = map.iter().fold(
        (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
        |(min_elevation, max_elevation, min_moisture, max_moisture), cell| {
            (
                min_elevation.min(cell.elevation),
                max_elevation.max(cell.elevation),
                min_moisture.min(cell.moisture),
                max_moisture.max(cell.moisture)
            )
        },
    );

    // normalize elevation and moisture
    let map = map.and_then(|cell| {
        let elevation = normalize(cell.elevation, min_elevation, max_elevation);
        let moisture = normalize(cell.moisture, min_moisture, max_moisture);

        Cell {
            elevation,
            moisture
        }
    });

    let map = map.and_then(|cell| {
        let (elevation, moisture) = (cell.elevation, cell.moisture);

        if elevation < sea_level {
            let normalized_elevation = normalize(elevation, 0.0, sea_level);

            ocean.lock().expect("to acquire the lock")(normalized_elevation)
        } else {
            evaluator.evaluate(elevation, moisture)
        }
    });
    let image = map.extract(pixel_map_to_image);

    image.save("image.png").expect("image to save");
}
