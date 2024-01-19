use anyhow::{anyhow, bail, Ok, Result};
use gilrs::Button;
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Describes a rectangle, inclusive.
struct Rect {
    x_start: usize,
    x_end: usize,
    y_start: usize,
    y_end: usize,
}

impl Rect {
    fn new(x_start: usize, x_end: usize, y_start: usize, y_end: usize) -> Result<Self> {
        if x_end < x_start || y_end < y_start {
            bail!("end must be greater or eq to start");
        }
        if x_start < 0 || y_start < 0 {
            bail!("invalid start value");
        }
        Ok(Self {
            x_start,
            x_end,
            y_start,
            y_end,
        })
    }

    fn top_left(self) -> Point {
        Point {
            x: self.x_start as i32,
            y: self.y_start as i32,
        }
    }

    fn top_right(self) -> Point {
        Point {
            x: self.x_end as i32,
            y: self.y_start as i32,
        }
    }

    fn bottom_left(self) -> Point {
        Point {
            x: self.x_start as i32,
            y: self.y_end as i32,
        }
    }

    fn bottom_right(self) -> Point {
        Point {
            x: self.x_end as i32,
            y: self.y_end as i32,
        }
    }
}

/// A point on the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn add(&self, x: i32, y: i32) -> Self {
        Point {
            x: self.x + x,
            y: self.y + y,
        }
    }
}

pub type LayoutID = String;
pub type FocusID = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpecialHandlerAction {
    NavigateOutRight, // Maybe maps to right shoulder button.
    NavigateOutLeft,  // Maybe maps to left shoulder button.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// For focus, we only handle these actions.
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn as_dir_vector(self) -> (i8, i8) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
    // Go sideways.
    fn as_side_dir_vectors(self) -> ((i8, i8), (i8, i8)) {
        match self {
            Direction::Up | Direction::Down => ((-1, 0), (1, 0)),
            Direction::Left | Direction::Right => ((0, -1), (0, 1)),
        }
    }
}

#[derive(Debug, Clone)]
struct Grid2D<T>
where
    T: Clone,
{
    x_size: usize,
    y_size: usize,
    grid: Vec<Vec<Option<T>>>,
}

impl<T> Grid2D<T>
where
    T: Clone,
{
    fn new(x_size: usize, y_size: usize) -> Result<Grid2D<T>> {
        if x_size <= 0 || y_size <= 0 {
            bail!("invalid grid size");
        }
        let mut v = Vec::new() as Vec<Vec<Option<T>>>;
        for _ in 0..x_size {
            let mut y = Vec::new();
            for _ in 0..y_size {
                y.push(None);
            }
            v.push(y);
        }
        Ok(Grid2D {
            x_size,
            y_size,
            grid: v,
        })
    }

    fn within_bounds(&self, x: i32, y: i32) -> bool {
        !(x >= self.x_size as i32 || x < 0 || y >= self.y_size as i32 || y < 0)
    }

    // Fill a rect area with item.
    fn fill(&mut self, rect: Rect, elem: T) -> Result<()> {
        if rect.x_end > self.x_size || rect.y_end > self.y_size {
            bail!("oversized rect detected");
        }
        // Range end is not inclusive.
        // Ensure the area is empty first.
        for x in rect.x_start..rect.x_end + 1 {
            for y in rect.y_start..rect.y_end + 1 {
                if self.grid[x][y].is_some() {
                    bail!("overlapping rect at {}, {}", x, y);
                }
            }
        }

        for x in rect.x_start..rect.x_end + 1 {
            for y in rect.y_start..rect.y_end + 1 {
                self.grid[x][y] = Some(elem.clone());
            }
        }
        Ok(())
    }

    // Get the element at a point.
    fn at(&self, x: usize, y: usize) -> Result<Option<T>> {
        if x >= self.x_size || y >= self.y_size {
            bail!("invalid coordinate {}, {}", x, y);
        }
        Ok(self.grid[x][y].clone())
    }
}

#[derive(Debug, Clone)]
struct LayoutGrid {
    grid: Grid2D<Arc<Mutex<GridItem>>>,
    layout_state: Option<Point>,
    special_handler: HashMap<Button, SpecialHandlerAction>,
    parent: Option<Weak<Mutex<LayoutGrid>>>,
    layout_id: LayoutID,
    sublayouts: HashMap<LayoutID, Weak<Mutex<GridItem>>>,
}

#[derive(Debug, Clone)]
/// A element in the grid.
pub enum GridItem {
    /// An element that is focusable.
    Element(FocusID, Rect),
    /// A sublayout grid.
    Sublayout(Arc<Mutex<LayoutGrid>>, Rect),
}

#[derive(Debug, Clone)]
enum NavigationDirective {
    Button(Button),
    Direction(Direction),
}

#[derive(Debug, Clone)]
enum NavigateAcrossBundle {
    NavigateToParent((f64, f64), NavigationDirective, LayoutID),
    NavigateToChild((f64, f64), NavigationDirective),
}

#[derive(Debug, Clone)]
enum NavigationResult {
    /// Navigation within the layout.
    WithinLayout(FocusID),
    /// Navigation across some layout, can be multiple layouts.
    AcrossLayout(FocusID, Weak<Mutex<LayoutGrid>>),
    /// Terminal.
    NoNextItem,
}

impl LayoutGrid {
    fn new(size_x: usize, size_y: usize, layout_id: LayoutID) -> Result<Self> {
        Ok(Self {
            grid: Grid2D::new(size_x, size_y)?,
            layout_state: None,
            special_handler: HashMap::new(),
            parent: None,
            layout_id: layout_id,
            sublayouts: HashMap::new(),
        })
    }
    /// Process a NavigationDirective and returns the next FocusID, with a
    /// weak reference to the next LayoutGrid.
    fn navigate(&mut self, directive: NavigationDirective) -> Result<NavigationResult> {
        // Check for special handler first.
        if let NavigationDirective::Button(b) = directive {
            if let Some(action) = self.special_handler.get(&b) {
                match action {
                    SpecialHandlerAction::NavigateOutRight => {
                        let corner = Point {
                            x: self.grid.x_size as i32 - 1,
                            y: 0,
                        };
                        self.set_point(corner.x as usize, corner.y as usize)?;
                        return self.navigate(NavigationDirective::Direction(Direction::Left));
                    }
                    SpecialHandlerAction::NavigateOutLeft => {
                        let corner = Point { x: 0, y: 0 };
                        self.set_point(corner.x as usize, corner.y as usize)?;
                        return self.navigate(NavigationDirective::Direction(Direction::Right));
                    }
                }
            }
        }

        // Grid navigation.
        // First, check if we are navigating out.
        if let NavigationDirective::Direction(d) = directive {
            // Set corner based on the direction.
            let corner = match d {
                Direction::Up | Direction::Left => self.current_item()?.1.top_left(),
                Direction::Down | Direction::Right => self.current_item()?.1.bottom_right(),
            };

            let (x_dir, y_dir) = d.as_dir_vector();
            // Only navigating out if we are at some edge.
            let mut next = corner.add(x_dir as i32, y_dir as i32);
            if !self.grid.within_bounds(next.x, next.y) {
                return self.try_navigate_out(&corner, directive);
            }

            // Otherwise, depending on the direction, look for the next possible
            // element in the grid.
            // Check for element in a line:
            while self.grid.within_bounds(next.x, next.y) {
                match self.try_navigate_to_loc(
                    next.x as usize,
                    next.y as usize,
                    directive.clone(),
                )? {
                    Some(s) => return Ok(s),
                    None => {
                        next = next.add(x_dir as i32, y_dir as i32);
                    }
                }
            }

            // Went out of bounds. Now, try to search sideways.
            next = corner.add(x_dir as i32, y_dir as i32);

            while self.grid.within_bounds(next.x, next.y) {
                // Try both side directions.
                let (dir_a, dir_b) = d.as_side_dir_vectors();

                for dir in vec![dir_a, dir_b] {
                    let mut dir_point = next.add(dir.0 as i32, dir.1 as i32);
                    while self.grid.within_bounds(dir_point.x, dir_point.y) {
                        // Check what's at loc.
                        // Prohibits sublayout when doing sideway navigation.
                        match self.grid.at(dir_point.x as usize, dir_point.y as usize)? {
                            Some(item) => match *item.clone().lock().unwrap() {
                                GridItem::Sublayout(..) => {
                                    continue;
                                }
                                _ => {}
                            },
                            None => {}
                        };

                        match self.try_navigate_to_loc(
                            dir_point.x as usize,
                            dir_point.y as usize,
                            directive.clone(),
                        )? {
                            Some(s) => return Ok(s),
                            None => {
                                dir_point = dir_point.add(dir.0 as i32, dir.1 as i32);
                            }
                        }
                    }
                }

                next = next.add(x_dir as i32, y_dir as i32);
            }

            return Ok(NavigationResult::NoNextItem);
        }

        unreachable!();
    }

    /// Try to navigate to a point.
    /// Returns Result<None> when the grid is empty at the point.
    fn try_navigate_to_loc(
        &mut self,
        x: usize,
        y: usize,
        directive: NavigationDirective,
    ) -> Result<Option<NavigationResult>> {
        match self.grid.at(x, y)? {
            Some(item) => match *item.clone().lock().unwrap() {
                GridItem::Element(ref focus_id, _) => {
                    self.set_point(x, y)?;
                    Ok(Some(NavigationResult::WithinLayout(focus_id.clone())))
                }
                GridItem::Sublayout(ref sublayout, rect) => {
                    // Calculate the x, y value relative to child.
                    let x_in = (x as i32 - rect.x_start as i32) as f64
                        / (rect.x_end as i32 - rect.x_start as i32) as f64;
                    let y_in = (y as i32 - rect.y_start as i32) as f64
                        / (rect.y_end as i32 - rect.y_start as i32) as f64;

                    match sublayout.lock().unwrap().navigate_into(
                        NavigateAcrossBundle::NavigateToChild((x_in, y_in), directive),
                    )? {
                        // Maps within layout to across layout.
                        NavigationResult::WithinLayout(s) => Ok(Some(
                            NavigationResult::AcrossLayout(s, Arc::downgrade(&sublayout)),
                        )),
                        // Respect deeper navigation results.
                        NavigationResult::AcrossLayout(s, w) => {
                            Ok(Some(NavigationResult::AcrossLayout(s, w)))
                        }
                        NavigationResult::NoNextItem => Ok(Some(NavigationResult::NoNextItem)),
                    }
                }
            },
            None => Ok(None),
        }
    }

    fn current_item(&self) -> Result<(FocusID, Rect)> {
        let curr_point = self.layout_state.ok_or(anyhow!("no layout state"))?;
        match self.grid.at(curr_point.x as usize, curr_point.y as usize)? {
            Some(elem) => match *elem.lock().unwrap() {
                GridItem::Element(ref id, ref rect) => Ok((id.clone(), rect.clone())),
                GridItem::Sublayout(ref layout, _) => bail!(
                    "layout id: {} is a sublayout, cannot set focus",
                    layout.lock().unwrap().layout_id
                ),
            },
            None => bail!("No element at {} {}", curr_point.x, curr_point.y),
        }
    }

    // Set the current point, for example, when first launch the application.
    // Can be invalid.
    fn set_point(&mut self, x: usize, y: usize) -> Result<()> {
        if !self.grid.within_bounds(x as i32, y as i32) {
            bail!("point {},{} is outside of the bounds", x, y)
        }
        self.layout_state = Some(Point {
            x: x as i32,
            y: y as i32,
        });
        Ok(())
    }

    // Navigate to the parent iff there is one.
    fn try_navigate_out(
        &mut self,
        out_from: &Point,
        directive: NavigationDirective,
    ) -> Result<NavigationResult> {
        // Try to find the parent.
        if let Some(p) = self.parent.clone() {
            if let Some(g) = p.upgrade() {
                // Calculate the out percentage.
                let x_out = out_from.x as f64 / self.grid.x_size as f64;
                let y_out = out_from.y as f64 / self.grid.y_size as f64;
                return match g
                    .lock()
                    .unwrap()
                    .navigate_into(NavigateAcrossBundle::NavigateToParent(
                        (x_out, y_out),
                        directive,
                        self.layout_id.clone(),
                    ))? {
                        // Maps within layout to across layout.
                        NavigationResult::WithinLayout(s) => Ok(
                            NavigationResult::AcrossLayout(s,p),
                        ),
                        // Respect deeper navigation results.
                        NavigationResult::AcrossLayout(s, w) => 
                            Ok(NavigationResult::AcrossLayout(s, w)),
                        NavigationResult::NoNextItem => Ok(NavigationResult::NoNextItem),
                    }
            }
        }
        // No parents.
        Ok(NavigationResult::NoNextItem)
    }

    /// Navigate across layouts.
    fn navigate_into(&mut self, bundle: NavigateAcrossBundle) -> Result<NavigationResult> {
        // Two possible cases, either we are navigating to parent, or
        // we are navigating to child.

        match bundle {
            // For child -> parent, child need to report the position it came out of
            // with reference to sublayout in the parent layout.
            NavigateAcrossBundle::NavigateToParent((exit_x, exit_y), directive, layout_id) => {
                // This is unholy.
                match self
                    .sublayouts
                    .get(&layout_id)
                    .ok_or(anyhow!("unexpected layout arrangement"))?
                    .upgrade()
                    .ok_or(anyhow!("unexpected result when getting child layout"))?
                    .lock()
                    .unwrap()
                    .to_owned()
                {
                    GridItem::Element(_, _) => {
                        bail!("unexpected element when looking for sublayout")
                    }
                    GridItem::Sublayout(_, rect) => {
                        // Calculate the new point relative to self.
                        self.set_point(
                            ((rect.x_end as f64 - rect.x_start as f64) * exit_x) as usize,
                            ((rect.y_end as f64 - rect.y_start as f64) * exit_y) as usize,
                        )?;
                    }
                }
                // Check if we landed on something.
                match self.try_navigate_to_loc(self.layout_state.unwrap().x as usize, self.layout_state.unwrap().y as usize, directive.clone())? {
                    Some(r) => return Ok(r),
                    None => {}
                }
                // If not, process the directive again within the child.
                self.navigate(directive)
            }
            // For parent -> child, parent need to tell the child the location of entry.
            NavigateAcrossBundle::NavigateToChild((in_x, in_y), directive) => {
                let x = self.grid.x_size * in_x as usize;
                let y = self.grid.y_size * in_y as usize;
                self.set_point(x, y)?;
                // Check if we landed on something.
                match self.try_navigate_to_loc(x, y, directive.clone())? {
                    Some(r) => return Ok(r),
                    None => {}
                }
                // If not, process the directive again within the child.
                self.navigate(directive)
            }
        }
    }
}

#[derive(Debug)]
pub struct LayoutGridBuilder {
    size_x: usize,
    size_y: usize,
    rects: Vec<(Rect, FocusID)>,
    sublayouts: Vec<(Rect, LayoutID, LayoutGridBuilder)>,
    layout_id: LayoutID,
    is_root_builder: bool,
}

impl LayoutGridBuilder {
    pub fn new(size_x: usize, size_y: usize, layout_id: LayoutID) -> Self {
        Self {
            size_x,
            size_y,
            rects: vec![],
            sublayouts: vec![],
            layout_id,
            is_root_builder: true,
        }
    }

    fn new_sub(size_x: usize, size_y: usize, layout_id: LayoutID) -> Self {
        Self {
            is_root_builder: false,
            ..LayoutGridBuilder::new(size_x, size_y, layout_id)
        }
    }

    fn add_element(&mut self, rect: Rect, focus_id: FocusID) -> &mut Self {
        self.rects.push((rect, focus_id));
        self
    }

    fn with_sublayout<'a>(
        &'a mut self,
        rect: Rect,
        layout_id: LayoutID,
        size_x: usize,
        size_y: usize,
    ) -> &'a mut Self {
        self.sublayouts.push((
            rect,
            layout_id.clone(),
            LayoutGridBuilder::new_sub(size_x, size_y, layout_id.clone()),
        ));
        self.sublayouts.last_mut().unwrap().2.borrow_mut()
    }

    pub fn build(self) -> Result<Arc<Mutex<LayoutGrid>>> {
        if !self.is_root_builder {
            bail!("must be called from the root builder");
        }

        self.build_sub(None)
    }

    fn build_sub(self, parent: Option<Weak<Mutex<LayoutGrid>>>) -> Result<Arc<Mutex<LayoutGrid>>> {
        let mut this_layout = LayoutGrid::new(self.size_x, self.size_y, self.layout_id)?;
        // Set parent.
        if let Some(ref parent_ref) = parent {
            this_layout.parent = Some(parent_ref.clone());
        }

        for (rect, focus_id) in self.rects {
            let e = Arc::new(Mutex::new(GridItem::Element(focus_id, rect)));
            this_layout.grid.fill(rect, e)?;
        }

        let this_layout_arc = Arc::new(Mutex::new(this_layout));
        for (sub_rect, sub_layout_id, sub_builder) in self.sublayouts {
            let sub_layout = sub_builder.build_sub(Some(Arc::downgrade(&this_layout_arc)))?;



            let e = Arc::new(Mutex::new(GridItem::Sublayout(sub_layout, sub_rect)));

            let mut ref_parent_layout = this_layout_arc.lock().unwrap();
            // Fill area with sublayouts too.
            ref_parent_layout.grid.fill(sub_rect, e.clone())?;
            // Now, add this sublayout to the parent map.
            ref_parent_layout
                .sublayouts
                .insert(sub_layout_id, Arc::downgrade(&e));
        }

        Ok(this_layout_arc)
    }
}

pub struct NavigationController {
    root_layout: Arc<Mutex<LayoutGrid>>,
    current_layout_ref: Weak<Mutex<LayoutGrid>>,
}

impl NavigationController {
    fn new(root_layout: Arc<Mutex<LayoutGrid>>) -> Self {
        let mut ret = Self {
            root_layout: root_layout.clone(),
            current_layout_ref: Arc::downgrade(&root_layout),
        };

        ret.root_layout.lock().unwrap().layout_state = Some(Point::default());
        ret
    }

    fn navigate(&mut self, directive: NavigationDirective) -> Result<NavigationResult> {
        match self
            .current_layout_ref
            .upgrade()
            .ok_or(anyhow!("unexpected result when getting layout"))?
            .lock()
            .unwrap()
            .navigate(directive)?
        {
            NavigationResult::WithinLayout(ref s) => {
                Ok(NavigationResult::WithinLayout(s.to_owned()))
            }
            NavigationResult::AcrossLayout(ref s, sub) => {
                self.current_layout_ref = sub.clone();
                Ok(NavigationResult::AcrossLayout(s.to_owned(), sub))
            }
            NavigationResult::NoNextItem => Ok(NavigationResult::NoNextItem),
        }
    }
}

// Conceptually, a layout can contain sublayouts in a grid.
// A sublayout can be entered or exited.
// For example, the scrollable games area in the home page is a sublayout.
// This sublayout have their own viewport and their offset can changes
// independent of the parent layout.
// When entering the sublayout, we need to calculate the point of entry wrt
// to the sublayout, taking the current viewport and offset into account .
// Similarly, when exiting the sublayout, we will calculate the exit point
// and find the next focus on the parent layout.

// We will also need to all kinds of button events.
// For example, a shoulder button can jump out directly of a sublayout when
// in a sublayout context, and the same button can behave the same as a regular
// direction button in the root context.

// Furthermore, when handling controller A or B buttons, we should
// perhaps forward the event back the UI via a callback? Alternatively,
// we will handle all the state changes in native code.
// I think the later is preferrable.

// The X, Y of the layout grid is arranged like so:
// -X----------------------------------------------------->
// |
// Y
// |
// |
// |
// |
// |
// |
// v

#[cfg(test)]
mod tests {
    use std::{assert_matches::assert_matches, borrow::Borrow};

    use super::*;

    fn simple_layout() -> Result<Arc<Mutex<LayoutGrid>>> {
        let mut builder = LayoutGridBuilder::new(10, 5, "L0".to_owned());
        builder
            .add_element(Rect::new(0, 1, 0, 1)?, "0_alpha".to_owned())
            .add_element(Rect::new(2, 2, 0, 1)?, "0_beta".to_owned());
        builder.build()
    }

    fn nested_layout() -> Result<Arc<Mutex<LayoutGrid>>> {
        let mut builder = LayoutGridBuilder::new(10, 5, "L0".to_owned());
        builder
            .add_element(Rect::new(0, 1, 0, 1)?, "0_alpha".to_owned())
            .add_element(Rect::new(2, 2, 0, 1)?, "0_beta".to_owned());
        builder
            .with_sublayout(Rect::new(0, 9, 2, 4)?, "L1".to_owned(), 7, 10)
            .add_element(Rect::new(0, 0, 0, 9)?, "1_alpha".to_owned())
            .add_element(Rect::new(1, 1, 0, 9)?, "1_beta".to_owned());

        builder.build()
    }

    fn element_at_is(layout: Arc<Mutex<LayoutGrid>>, x: usize, y: usize, g: &GridItem) {
        let m = layout.lock().unwrap();
        let elem = m.grid.at(x, y).unwrap();

        assert_matches!(elem, Some(_));
        assert_matches!(g, GridItem::Element(..));

        if let GridItem::Element(ref expected_s, ref expected_r) = g {
            if let GridItem::Element(ref s, ref r) = *elem.unwrap().lock().unwrap() {
                assert_eq!(s, expected_s);
                assert_eq!(r, expected_r);
            } else {
                panic!("bad element {:?}", m)
            }
        } else {
            panic!("invalid grid item input")
        }
    }

    fn element_in_rect_is(layout: Arc<Mutex<LayoutGrid>>, rect: &Rect, g: &GridItem) {
        for x in rect.x_start..rect.x_end + 1 {
            for y in rect.y_start..rect.y_end + 1 {
                element_at_is(layout.clone(), x, y, g)
            }
        }
    }

    #[test]
    fn sample_grid_has_expected_items() {
        let sut = simple_layout().unwrap();

        element_in_rect_is(
            sut.clone(),
            &Rect::new(0, 1, 0, 1).unwrap(),
            &GridItem::Element("0_alpha".to_owned(), Rect::new(0, 1, 0, 1).unwrap()),
        );

        element_in_rect_is(
            sut.clone(),
            &Rect::new(2, 2, 0, 1).unwrap(),
            &GridItem::Element("0_beta".to_owned(), Rect::new(2, 2, 0, 1).unwrap()),
        );
    }

    #[test]
    fn can_build_nested_layout() {
        nested_layout().unwrap();
    }

    mod navigation_controller_test {
        use super::*;
        use std::{assert_matches::assert_matches, borrow::Borrow};

        #[test]
        fn can_build_controller() {
            NavigationController::new(nested_layout().unwrap());
        }

        #[test]
        fn navigation_right() {
            let mut controller = NavigationController::new(nested_layout().unwrap());
            let mut res = controller
                .navigate(NavigationDirective::Direction(Direction::Right))
                .unwrap();
            if let NavigationResult::WithinLayout(ref id) = res {
                assert_eq!(id, "0_beta");
            } else {
                panic!("unexpected navigation result {:?}", res)
            }

            res = controller
                .navigate(NavigationDirective::Direction(Direction::Right))
                .unwrap();
            assert_matches!(res, NavigationResult::NoNextItem);
        }

        #[test]
        fn navigation_into_sublayout() {
            let mut controller = NavigationController::new(nested_layout().unwrap());
            let mut res = controller
                .navigate(NavigationDirective::Direction(Direction::Down))
                .unwrap();
            if let NavigationResult::AcrossLayout(ref id, _) = res {
                assert_eq!(id, "1_alpha");
            } else {
                panic!("unexpected navigation result {:?}", res)
            }
        }

        #[test]
        fn navigation_into_sublayout_then_out() {
            let mut controller = NavigationController::new(nested_layout().unwrap());
            let mut res = controller
                .navigate(NavigationDirective::Direction(Direction::Down))
                .unwrap();
            if let NavigationResult::AcrossLayout(ref id, _) = res {
                assert_eq!(id, "1_alpha");
            } else {
                panic!("unexpected navigation result {:?}", res)
            }

            res = controller
                .navigate(NavigationDirective::Direction(Direction::Up))
                .unwrap();
            if let NavigationResult::AcrossLayout(ref id, _) = res {
                assert_eq!(id, "0_alpha");
            } else {
                panic!("unexpected navigation result {:?}", res)
            }
        }
    }
}
