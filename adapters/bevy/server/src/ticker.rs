pub struct Ticker {
    ticked: bool,
}

impl Ticker {
    pub fn new() -> Self {
        Self { ticked: false }
    }

    pub fn tick_start(&mut self) {
        self.ticked = true;
    }

    pub fn tick_finish(&mut self) {
        self.ticked = false;
    }

    pub fn has_ticked(&self) -> bool {
        return self.ticked;
    }
}
