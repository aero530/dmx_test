impl :: bincode :: Encode for PwmSettings
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode :: encode(&self.freq, encoder) ?; core :: result
        :: Result :: Ok(())
    }
}