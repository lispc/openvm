use p3_field::Field;

pub trait VmMemory<const WORD_SIZE: usize, F: Field> {
    fn read_word(&mut self, timestamp: usize, address_space: F, address: F) -> [F; WORD_SIZE];
    fn write_word(&mut self, timestamp: usize, address_space: F, address: F, data: [F; WORD_SIZE]);

    /// Reads a word directly from memory without updating internal state.
    ///
    /// Any value returned is unconstrained.
    fn unsafe_read_word(&self, address_space: F, address: F) -> [F; WORD_SIZE];

    fn compose(word: [F; WORD_SIZE]) -> F;
    fn decompose(field_elem: F) -> [F; WORD_SIZE];
    fn read_elem(&mut self, timestamp: usize, address_space: F, address: F) -> F {
        Self::compose(self.read_word(timestamp, address_space, address))
    }

    fn write_elem(&mut self, timestamp: usize, address_space: F, address: F, data: F) {
        self.write_word(timestamp, address_space, address, Self::decompose(data));
    }

    /// Reads an element directly from memory without updating internal state.
    ///
    /// Any value returned is unconstrained.
    fn unsafe_read_elem(&self, address_space: F, address: F) -> F {
        Self::compose(self.unsafe_read_word(address_space, address))
    }
}
