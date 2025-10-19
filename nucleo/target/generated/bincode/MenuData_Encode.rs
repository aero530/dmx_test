impl :: bincode :: Encode for MenuData
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode :: encode(&self.dmx_address, encoder) ?; ::
        bincode :: Encode :: encode(&self.module, encoder) ?; core :: result
        :: Result :: Ok(())
    }
}