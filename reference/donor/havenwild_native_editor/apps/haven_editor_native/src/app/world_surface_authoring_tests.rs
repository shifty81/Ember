    use super::*;

    #[test]
    fn global_grid_line_keeps_dragged_roads_continuous() {
        let cells = global_grid_line(GridPos { x: 2, y: 3 }, GridPos { x: 7, y: 5 });
        assert_eq!(cells.first(), Some(&GridPos { x: 2, y: 3 }));
        assert_eq!(cells.last(), Some(&GridPos { x: 7, y: 5 }));
        assert!(cells.windows(2).all(|pair| {
            let dx = (pair[1].x - pair[0].x).abs();
            let dy = (pair[1].y - pair[0].y).abs();
            dx <= 1 && dy <= 1 && dx + dy > 0
        }));
    }
