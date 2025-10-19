impl :: bincode :: Encode for EepromImage
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode :: encode(&self.module, encoder) ?; :: bincode ::
        Encode :: encode(&self.settings, encoder) ?; core :: result :: Result
        :: Ok(())
    }
}