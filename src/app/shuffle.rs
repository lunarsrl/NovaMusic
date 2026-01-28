use crate::app::AppTrack;

use rand::seq::SliceRandom;

pub fn shuffle_queue(queue: &mut Vec<AppTrack>, queue_pos: &mut usize) {
    if queue.len() <= 1 {
        return;
    }

    let current = queue.remove(*queue_pos);
    let mut rng = rand::rng();
    queue.shuffle(&mut rng);
    queue.insert(0, current);
    *queue_pos = 0;
}
