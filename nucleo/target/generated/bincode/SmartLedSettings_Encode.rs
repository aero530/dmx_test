impl :: bincode :: Encode for SmartLedSettings
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        :: bincode :: Encode :: encode(&self.leds_per_port, encoder) ?; ::
        bincode :: Encode :: encode(&self.color_mode, encoder) ?; :: bincode
        :: Encode :: encode(&self.grouping, encoder) ?; core :: result ::
        Result :: Ok(())
    }
}