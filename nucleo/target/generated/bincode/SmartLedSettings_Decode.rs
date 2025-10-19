impl < __Context > :: bincode :: Decode < __Context > for SmartLedSettings
{
    fn decode < __D : :: bincode :: de :: Decoder < Context = __Context > >
    (decoder : & mut __D) ->core :: result :: Result < Self, :: bincode ::
    error :: DecodeError >
    {
        core :: result :: Result ::
        Ok(Self
        {
            leds_per_port : :: bincode :: Decode :: decode(decoder) ?,
            color_mode : :: bincode :: Decode :: decode(decoder) ?, grouping :
            :: bincode :: Decode :: decode(decoder) ?,
        })
    }
} impl < '__de, __Context > :: bincode :: BorrowDecode < '__de, __Context >
for SmartLedSettings
{
    fn borrow_decode < __D : :: bincode :: de :: BorrowDecoder < '__de,
    Context = __Context > > (decoder : & mut __D) ->core :: result :: Result <
    Self, :: bincode :: error :: DecodeError >
    {
        core :: result :: Result ::
        Ok(Self
        {
            leds_per_port : :: bincode :: BorrowDecode ::< '_, __Context >::
            borrow_decode(decoder) ?, color_mode : :: bincode :: BorrowDecode
            ::< '_, __Context >:: borrow_decode(decoder) ?, grouping : ::
            bincode :: BorrowDecode ::< '_, __Context >::
            borrow_decode(decoder) ?,
        })
    }
}