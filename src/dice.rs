use rand::Rng;

pub fn roll_dice(dice: i32, sides: i32) -> Vec<i32> {
    // Generate a random index
    let mut rng = rand::thread_rng();
    let mut rolls = Vec::new();

    for _ in 0..dice {
        rolls.push(rng.gen_range(1..sides));
    }

    rolls
}
