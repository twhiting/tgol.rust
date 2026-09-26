use super::Grid;

#[test]
fn stationary_block_reports_no_change() {
    let mut grid = Grid::new_empty(6, 6);
    for (x, y) in [(2, 2), (3, 2), (2, 3), (3, 3)] {
        let index = grid.grid_idx(x, y).unwrap();
        grid.grid[index].set(true);
    }
    grid.reset_cycle_detection();

    assert!(!grid.update());
}

#[test]
fn repeating_oscillator_eventually_reports_no_change() {
    let mut grid = Grid::new_empty(7, 7);
    for (x, y) in [(3, 2), (3, 3), (3, 4)] {
        let index = grid.grid_idx(x, y).unwrap();
        grid.grid[index].set(true);
    }
    grid.reset_cycle_detection();

    assert!(grid.update());
    assert!(grid.update());
    assert!(!grid.update());
}
