use rand::{self, Rng};
use std::io;
use std::str::FromStr;
use std::sync::RwLock;

static GLOBAL_STATE: RwLock<GameState> = RwLock::new(GameState::new());

#[derive(Clone, Copy)]
struct GameState {
    picked_tile: bool,
    game_ended: bool,
    game_won: bool,
    game_ending_tile: Option<(usize, usize)>,
}

impl GameState {
    const fn new() -> Self {
        return Self {
            picked_tile: false,
            game_ended: false,
            game_won: false,
            game_ending_tile: None,
        };
    }

    fn restart(&mut self) {
        *self = Self::new();
    }
}

#[derive(Clone, Debug)]
struct Board {
    rows: usize,
    cols: usize,
    mines_count: usize,
    tiles: Vec<Tile>,
}

impl Board {
    fn new(rows: usize, cols: usize, mines: usize) -> Self {
        assert!((rows * cols) >= mines);

        let mut tiles = Vec::with_capacity(rows * cols);

        unsafe {
            tiles.set_len(tiles.capacity());
        };

        tiles.fill(Tile::new());

        Self {
            rows,
            cols,
            mines_count: mines,
            tiles,
        }
    }

    // reveals mines and adjacent mines if the revealed mine is not surrounded by any mines
    pub fn reveal(&mut self, x: usize, y: usize) {
        let tile_idx = (y * self.cols) + x;

        if !GLOBAL_STATE.read().unwrap().picked_tile {
            if self.tiles[tile_idx].has_mine {
                println!(
                    "mine_count: {}",
                    self.tiles.iter().filter(|tile| tile.has_mine).count()
                );

                // Todo: move mine
                let (mut new_x, mut new_y) = self.get_new_pos();
                while new_x == x && new_y == y {
                    (new_x, new_y) = self.get_new_pos();
                }

                let tile_a = self.tiles[(new_y * self.cols) + new_x];
                let tile_b = self.tiles[tile_idx];

                self.tiles[tile_idx] = tile_a;
                self.tiles[(new_y * self.cols) + new_x] = tile_b;

                return self.reveal(x, y);
            }

            GLOBAL_STATE.write().unwrap().picked_tile = true;
        }

        let tile = &mut self.tiles[tile_idx];

        if tile.revealed {
            return;
        }

        tile.reveal();

        if GLOBAL_STATE.read().unwrap().game_ended {
            GLOBAL_STATE.write().unwrap().game_ending_tile = Some((x, y));
            return;
        }

        let neighbors_pos = self.get_neighboring_tiles(x, y);

        // if any neighboring tiles have a mine, stop the recursive reveal
        if neighbors_pos
            .iter()
            .filter(|(tile_x, tile_y)| {
                let tile = &self.tiles[(tile_y * self.cols) + tile_x];
                tile.has_mine
            })
            .count()
            > 0
        {
            return;
        }

        for (tile_x, tile_y) in neighbors_pos {
            self.reveal(tile_x, tile_y);
        }
    }

    pub fn flag(&mut self, x: usize, y: usize) {
        let tile_idx = (y * self.cols) + x;
        let tile = &mut self.tiles[tile_idx];

        tile.flag();
    }

    pub fn remaining_tiles(&self) -> usize {
        return self
            .tiles
            .iter()
            .filter(|tile| !tile.revealed && !tile.has_flag)
            .count();
    }

    pub fn flagged_tiles(&self) -> usize {
        return self.tiles.iter().filter(|tile| tile.has_flag).count();
    }

    // TODO: gave save us all from these horrible names
    fn get_neighboring_tiles(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut neighbors: Vec<(usize, usize)> = Vec::new();
        // dumb solution, yes? Do I care, not really.
        let tl_x = x as isize - 1;
        let tl_y = y as isize - 1;

        for tile_x in 0..3 {
            for tile_y in 0..3 {
                let check_x = tl_x + tile_x;
                let check_y = tl_y + tile_y;

                if (check_x < 0 || check_y < 0)
                    || (check_x > self.cols as isize - 1 || check_y > self.rows as isize - 1)
                {
                    continue;
                }

                if check_x as usize == x && check_y as usize == y {
                    // Don't include outselfs in the check obv
                    continue;
                }

                neighbors.push((check_x as usize, check_y as usize));
            }
        }

        neighbors
    }

    pub fn flag_all_remaining_tiles(&mut self) {
        if self.remaining_tiles() + self.flagged_tiles() != self.mines_count {
            panic!("Auto win, but there are more tiles than mines");
        }

        let mine_tiles = self
            .tiles
            .iter_mut()
            .filter(|tile| !tile.has_flag && !tile.revealed)
            .collect::<Vec<_>>();

        for tile in mine_tiles {
            tile.flag();
        }
    }

    pub fn draw(&self) {
        let mine_count = self.mines_count - self.tiles.iter().filter(|tile| tile.has_flag).count();
        // use the max function because if the number is 0, the digits are 0
        let digits = f64::max(f64::floor(f64::log10(mine_count as f64) + 1.0), 1.0) as usize;

        println!("   ┌{}┐", "─".repeat(self.cols));
        println!(
            "   │{string:<width$}{mine_count} │",
            width = (self.cols - (digits + 1)),
            string = " ".repeat(self.cols - (digits + 1)),
            mine_count = mine_count
        );
        println!("   ├{}┤", "─".repeat(self.cols));

        for row in 0..self.rows {
            for col in 0..self.cols {
                if col == 0 {
                    print!("{row:<02} │");
                }

                let tile = self.tiles[(row * self.cols) + col];

                if GLOBAL_STATE.read().unwrap().game_ended && tile.has_mine {
                    // Highlight the game ending mine in red, and the flagged mines in green
                    print!(
                        "{}*\x1B[0m",
                        if tile.has_flag {
                            "\x1B[92m"
                        } else if {
                            let game_ending_tile =
                                GLOBAL_STATE.read().unwrap().game_ending_tile.unwrap();
                            game_ending_tile.0 == col && game_ending_tile.1 == row
                        } {
                            "\x1B[91m"
                        } else {
                            ""
                        }
                    );

                    if col == self.cols - 1 {
                        print!("│");
                    }

                    continue;
                }

                if tile.revealed {
                    let nearby_mines = self
                        .get_neighboring_tiles(col, row)
                        .iter()
                        .filter(|(tile_x, tile_y)| {
                            let tile = &self.tiles[(tile_y * self.cols) + tile_x];
                            tile.has_mine
                        })
                        .count();

                    if nearby_mines == 0 {
                        print!(" ");
                    } else {
                        print!(
                            "{}{nearby_mines}\x1B[0m",
                            match nearby_mines {
                                1 => "\x1B[32m",
                                2 => "\x1B[37m",
                                3 => "\x1B[96m",
                                4 => "\x1B[33m",
                                5 => "\x1B[34m",
                                6 => "\x1B[35m",
                                7 => "\x1B[31m",
                                8 => "\x1B[97m",
                                _ => panic!("More than 8 nearby mines!"),
                            }
                        );
                    }
                } else if tile.has_flag {
                    if GLOBAL_STATE.read().unwrap().game_ended {
                        print!("\x1B[91m^\x1B0m");
                    } else {
                        print!("^");
                    }
                } else {
                    print!("#");
                }

                if col == self.cols - 1 {
                    print!("│");
                }
            }

            println!("");
        }

        println!("   └{}┘", "─".repeat(self.cols));
    }

    pub fn generate_mines(&mut self) {
        for _ in 0..self.mines_count {
            let pos = self.get_new_pos();

            let idx = (pos.1 * self.cols) + pos.0;

            self.tiles[idx].has_mine = true;
        }
    }

    fn get_new_pos(&self) -> (usize, usize) {
        let mut y = rng(self.rows);
        let mut x = rng(self.cols);
        let mut idx = (y * self.cols) + x;

        while self.tiles[idx].has_mine != false {
            y = rng(self.rows);
            x = rng(self.cols);

            idx = (y * self.cols) + x;
        }

        return (x, y);
    }
}

#[derive(Clone, Copy, Debug)]
struct Tile {
    pub revealed: bool,
    pub has_mine: bool,
    pub has_flag: bool,
}

impl Tile {
    fn new() -> Self {
        Self {
            revealed: false,
            has_mine: false,
            has_flag: false,
        }
    }

    fn reveal(&mut self) {
        if self.has_mine {
            GLOBAL_STATE.write().unwrap().game_ended = true;
            return;
        }

        self.has_flag = false;
        self.revealed = true;
    }

    fn flag(&mut self) {
        self.has_flag = !self.has_flag;
    }
}

fn main() -> io::Result<()> {
    print!("\x1B[2J\x1B[1;1H");
    println!("Starting RustySweep... (bad name ik)");

    loop {
        GLOBAL_STATE.write().unwrap().restart();

        game_loop()?;
    }
}

fn game_loop() -> io::Result<()> {
    let mut stdin = String::new();

    println!("Please select board size");

    let boards = [
        Board::new(9, 9, 10),
        Board::new(16, 16, 40),
        Board::new(16, 30, 99),
    ];

    for (i, board) in boards.iter().enumerate() {
        println!(
            "[{}]: {:<02}x{:<02} {:<02}",
            i + 1,
            board.rows,
            board.cols,
            board.mines_count
        );
    }

    println!("[{}]: Custom board", boards.len() + 1);

    io::stdin().read_line(&mut stdin)?;

    // easier if we want to make custom sized boards a thing later
    let mut board = match usize::from_str(&stdin.trim()) {
        Err(_) => {
            println!("Selected board is invalid!");

            return Ok(());
        }

        Ok(idx) => {
            if idx == boards.len() + 1 {
                // custom board
                let custom_board = make_custom_board();

                let board = match custom_board {
                    Ok(board) => board,
                    Err(board_err) => match board_err {
                        CustomBoardError::Cancel => return Ok(()),
                        CustomBoardError::Error => {
                            println!("Invalid custom board!");
                            return Ok(());
                        }
                    },
                };

                board
            } else {
                if idx > boards.len() {
                    println!("Selected board is invalid!");

                    return Ok(());
                }

                boards[idx - 1].clone()
            }
        }
    };

    drop(stdin);

    board.generate_mines();

    loop {
        print!("\x1B[2J\x1B[1;1H");

        if board.remaining_tiles() == 0 {
            GLOBAL_STATE.write().unwrap().game_won = true;
        }

        if board.remaining_tiles() == board.mines_count
            || (board.remaining_tiles() + board.flagged_tiles()) == board.mines_count
        {
            board.flag_all_remaining_tiles();
            GLOBAL_STATE.write().unwrap().game_won = true;
        }

        board.draw();

        if GLOBAL_STATE.read().unwrap().game_won {
            println!("You won!");
            enter_to_continue();

            break;
        }

        if GLOBAL_STATE.read().unwrap().game_ended {
            println!("Game over!");
            enter_to_continue();

            break;
        }

        let x = get_input_num("Select an X position", None);

        if x.is_cancel() || x.is_invalid() {
            continue;
        }

        if x.get_num() >= board.cols {
            println!("X position is invalid!");
            enter_to_continue();

            continue;
        }

        let y = get_input_num("Select a Y position", None);

        if y.is_cancel() || x.is_invalid() {
            continue;
        }

        if y.get_num() >= board.rows {
            println!("Y position is invalid!");
            enter_to_continue();

            continue;
        }

        let action = get_input_num(
            "What would you like to do",
            Some(&["Reveal the tile", "Place or remove a flag"]),
        );

        if action.is_cancel() {
            continue;
        }

        let action: Action = ((action.get_num() as u8) - 1).into();

        println!("User action: {action:?}ing {} {}", x.get_num(), y.get_num());

        match action {
            Action::Reveal => board.reveal(x.get_num(), y.get_num()),
            Action::Flag => board.flag(x.get_num(), y.get_num()),
        }

        println!(
            "Tile at {} {} is now: {:?}",
            x.get_num(),
            y.get_num(),
            board.tiles[(y.get_num() * board.cols) + x.get_num()]
        );
    }

    Ok(())
}

fn enter_to_continue() {
    println!("Press Return to continue!");
    let _ = io::stdin().read_line(&mut String::new());
}

enum CustomBoardError {
    Cancel,
    Error,
}

fn make_custom_board() -> Result<Board, CustomBoardError> {
    let mut cols = get_input_num("Board width", None);

    if cols.is_cancel() {
        return Err(CustomBoardError::Cancel);
    }

    if cols.is_invalid() {
        return Err(CustomBoardError::Error);
    }

    let mut rows = get_input_num("Board height", None);

    if rows.is_cancel() {
        return Err(CustomBoardError::Cancel);
    }

    if rows.is_invalid() {
        return Err(CustomBoardError::Error);
    }

    let mut mines = get_input_num("Number of mines", None);

    if mines.is_cancel() {
        return Err(CustomBoardError::Cancel);
    }

    if mines.is_invalid() {
        return Err(CustomBoardError::Error);
    }

    // winmine minimums at least in the winmine from archive.org
    if cols.get_num() < 8 {
        cols = Input::Num(8)
    }

    if rows.get_num() < 8 {
        rows = Input::Num(8);
    }

    if mines.get_num() < 10 {
        mines = Input::Num(10);
    }

    if mines.get_num() > (rows.get_num() - 1) * (cols.get_num() - 1) {
        mines = Input::Num((rows.get_num() - 1) * (cols.get_num() - 1));
    }

    return Ok(Board::new(rows.get_num(), cols.get_num(), mines.get_num()));
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum Action {
    Reveal,
    Flag,
}

impl From<u8> for Action {
    fn from(other: u8) -> Self {
        match other {
            0 => Self::Reveal,
            1 => Self::Flag,
            _ => panic!("Invalid Action {other}!"),
        }
    }
}

enum Input {
    Num(usize),
    Cancel,
    Invalid,
}

impl Input {
    fn is_cancel(&self) -> bool {
        match self {
            Input::Cancel => return true,
            _ => return false,
        }
    }

    fn get_num(&self) -> usize {
        match self {
            Input::Num(num) => return num.clone(),
            _ => panic!("tried to unwrap a non-Num value!"),
        }
    }

    fn is_invalid(&self) -> bool {
        match self {
            Input::Invalid => return true,
            _ => return false,
        }
    }
}

fn get_input_num(message: &str, options: Option<&[&str]>) -> Input {
    println!("{message} (c to cancel):");

    if options.is_some() {
        for (i, option) in options.unwrap().iter().enumerate() {
            println!("[{}]: {option}", i + 1);
        }
    }

    let mut stdin = String::new();

    io::stdin().read_line(&mut stdin).unwrap();
    if stdin.as_bytes()[0] == b'c' {
        return Input::Cancel;
    }

    let num = match usize::from_str(&stdin.trim()) {
        Err(_) => {
            return Input::Invalid;
        }

        Ok(idx) => {
            if options.is_some() {
                if idx > options.unwrap().len() {
                    return Input::Invalid;
                }
            }

            idx
        }
    };

    return Input::Num(num);
}

fn rng(max: usize) -> usize {
    rand::thread_rng().gen_range(0..max)
}
