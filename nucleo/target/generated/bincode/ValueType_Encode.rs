impl :: bincode :: Encode for ValueType
{
    fn encode < __E : :: bincode :: enc :: Encoder >
    (& self, encoder : & mut __E) ->core :: result :: Result < (), :: bincode
    :: error :: EncodeError >
    {
        match self
        {
            Self ::None
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (0u32), encoder) ?
                ; core :: result :: Result :: Ok(())
            }, Self ::U8(field_0)
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (1u32), encoder) ?
                ; :: bincode :: Encode :: encode(field_0, encoder) ?; core ::
                result :: Result :: Ok(())
            }, Self ::SmartLedGrouping(field_0)
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (2u32), encoder) ?
                ; :: bincode :: Encode :: encode(field_0, encoder) ?; core ::
                result :: Result :: Ok(())
            }, Self ::SmartLedColorMode(field_0)
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (3u32), encoder) ?
                ; :: bincode :: Encode :: encode(field_0, encoder) ?; core ::
                result :: Result :: Ok(())
            }, Self ::Uint3(field_0)
            =>{
                < u32 as :: bincode :: Encode >:: encode(& (4u32), encoder) ?
                ; :: bincode :: Encode :: encode(field_0, encoder) ?; core ::
                result :: Result :: Ok(())
            },
        }
    }
}