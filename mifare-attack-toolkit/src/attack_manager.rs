use std::error::Error;

use crate::reader::MifareClassic;
use crate::mifare_attack_manager::MifareAttackManager;

pub struct AttackManager<'a> {
    reader: &'a mut MifareClassic,
}

impl<'a> AttackManager<'a> {
    pub fn new(reader: &'a mut MifareClassic) -> Self {
        Self { reader }
    }
    
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut manager = MifareAttackManager::new(self.reader);
        manager.run()
    }
}
