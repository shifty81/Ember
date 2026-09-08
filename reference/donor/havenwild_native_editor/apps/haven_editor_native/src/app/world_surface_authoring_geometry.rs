use super::*;

pub(super) fn global_grid_line(start: GridPos, end: GridPos) -> Vec<GridPos> {
    let mut points = Vec::new();
    let mut x = start.x;
    let mut y = start.y;
    let dx = (end.x - start.x).abs();
    let sx = if start.x < end.x { 1 } else { -1 };
    let dy = -(end.y - start.y).abs();
    let sy = if start.y < end.y { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        points.push(GridPos { x, y });
        if x == end.x && y == end.y {
            break;
        }
        let doubled = error * 2;
        if doubled >= dy {
            error += dy;
            x += sx;
        }
        if doubled <= dx {
            error += dx;
            y += sy;
        }
    }
    points
}
