// ---------- src/lib/mifare/magic/detect/types.rs ----------
// Data structures for detection results

/// Result of a specific test
#[derive(Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub score: u32,
    pub notes: Vec<String>,
}



impl TestResult {
    pub fn new(name: &str) -> Self {
        TestResult {
            name: String::from(name),
            passed: false,
            score: 0,
            notes: Vec::new(),
        }
    }
    
    pub fn add_note(&mut self, note: &str) {
        self.notes.push(String::from(note));
    }
    
    pub fn set_passed(&mut self, score: u32) {
        self.passed = true;
        self.score += score;
    }
}

/// Overall detection results
pub struct DetectionResult {
    pub tests: Vec<TestResult>,
    pub total_score: u32,
    pub magic_card: bool,
}

impl DetectionResult {
    pub fn new() -> Self {
        DetectionResult {
            tests: Vec::new(),
            total_score: 0,
            magic_card: false,
        }
    }
    
// Then modify the add_test method to take a reference instead of ownership
pub fn add_test(&mut self, test: &TestResult) {
    self.total_score += test.score;
    self.tests.push(test.clone());
}    
    pub fn get_all_notes(&self) -> Vec<String> {
        self.tests.iter()
            .flat_map(|test| test.notes.clone())
            .collect()
    }
    
    pub fn has_passing_test(&self, test_name: &str) -> bool {
        self.tests.iter()
            .any(|test| test.name == test_name && test.passed)
    }
}
