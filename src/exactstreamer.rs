pub struct ExactStreamer<T> {
    remainder: Vec<T>,
    remainder_len: usize,
    receiver: crossbeam_channel::Receiver<Vec<T>>,
}

impl<T> ExactStreamer<T>
where
    T: Copy + Default,
{
    pub fn new(
        remainder_buffer_size: usize,
        receiver: crossbeam_channel::Receiver<Vec<T>>,
    ) -> ExactStreamer<T> {
        ExactStreamer {
            remainder: vec![T::default(); remainder_buffer_size],
            remainder_len: 0,
            receiver,
        }
    }

    pub fn fill(&mut self, out: &mut [T]) -> Result<(), crossbeam_channel::RecvError> {
        let mut i = self.remainder_len.min(out.len());

        out[..i].copy_from_slice(&self.remainder[..i]);

        // move old data to index 0 for next read
        self.remainder.copy_within(i..self.remainder_len, 0);
        self.remainder_len -= i;

        while i < out.len() {
            let generated = self.receiver.recv()?;

            if generated.len() > out.len() - i {
                let left = out.len() - i;
                out[i..].copy_from_slice(&generated[..left]);

                self.remainder_len = generated.len() - left;

                let vec_len = self.remainder.len();
                if vec_len < self.remainder_len {
                    self.remainder
                        .extend(std::iter::repeat(T::default()).take(self.remainder_len - vec_len));
                }

                self.remainder[..self.remainder_len].copy_from_slice(&generated[left..]);
                break;
            } else {
                out[i..(i + generated.len())].copy_from_slice(&generated);
                i += generated.len();
            }
        }

        Ok(())
    }
}
