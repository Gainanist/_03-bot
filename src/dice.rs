use bevy_turborand::RngComponent;


fn choose<'a, T>(rng: &mut RngComponent, items: &'a [T]) -> Option<&'a T> {
    if items.len() == 0 {
        None
    } else {
        Some(&items[rng.usize(items.len())])
    }
}

fn choose_mut<'a, T>(rng: &mut RngComponent, items: &'a mut [T]) -> Option<&'a mut T> {
    if items.len() == 0 {
        None
    } else {
        Some(&mut items[rng.usize(items.len())])
    }
}
