use crate::Coil;

const COILS_PER_BYTE: usize = 8;

/// Calculate the number of bytes needed to store the given number of coils
pub const fn bytes_needed(coils: usize) -> usize {
    (coils + COILS_PER_BYTE - 1) / COILS_PER_BYTE
}

/// Write coil values to the given byte slice
///
/// Any unneeded bytes will be left unchanged.
///
/// # Panics
///
/// Panics if there are not enough bytes in the `bytes` slice to support the
/// given number of coils. You can use `bytes_needed` to ensure you pass a
/// sufficiently large slice.
pub fn pack_coils(coils: &[Coil], bytes: &mut [u8]) {
    // Only consider the bytes in the range of values we need
    let bytes = &mut bytes[..bytes_needed(coils.len())];

    // Zero out output range
    for byte in &mut bytes[..] {
        *byte = 0;
    }

    for (coil_index, coil) in coils.iter().enumerate() {
        // The byte this coil will be assigned to
        let byte_index = coil_index / COILS_PER_BYTE;

        // The bit within the byte this coil will be assigned to
        let bit_index = coil_index % COILS_PER_BYTE;

        let bit_flag: u8 = 1 << bit_index;

        // If it's on, set the bit flag
        // If it's off, clear the bit flag
        match coil {
            Coil::On => bytes[byte_index] |= bit_flag,
            Coil::Off => bytes[byte_index] &= !bit_flag,
        }
    }
}

/// Unpack the given bytes into the given coil slice
///
/// The length of the `coils` slice drives the number of bytes that will be
/// decoded.
///
/// # Panics
///
/// Panics if there are not enough bytes in the `bytes` slice to support the
/// number of coils requested. You can use `bytes_needed` to ensure you pass
/// a sufficiently large slice.
pub fn unpack_coils(bytes: &[u8], coils: &mut [Coil]) {
    for (coil_index, coil) in coils.iter_mut().enumerate() {
        // The byte this coil is specified in
        let byte_index = coil_index / COILS_PER_BYTE;

        // The bit this coil is specified in
        let bit_index = coil_index % COILS_PER_BYTE;

        let bit_flag: u8 = 1 << bit_index;

        // If the flag is cleared, the coil is off. Otherwise, it's on
        *coil = if bytes[byte_index] & bit_flag == 0 {
            Coil::Off
        } else {
            Coil::On
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bytes_needed_works() {
        assert_eq!(bytes_needed(0), 0);
        assert_eq!(bytes_needed(1), 1);
        assert_eq!(bytes_needed(2), 1);
        assert_eq!(bytes_needed(3), 1);
        assert_eq!(bytes_needed(4), 1);
        assert_eq!(bytes_needed(5), 1);
        assert_eq!(bytes_needed(6), 1);
        assert_eq!(bytes_needed(7), 1);
        assert_eq!(bytes_needed(8), 1);
        assert_eq!(bytes_needed(9), 2);
        assert_eq!(bytes_needed(10), 2);
        assert_eq!(bytes_needed(11), 2);
        assert_eq!(bytes_needed(12), 2);
        assert_eq!(bytes_needed(13), 2);
        assert_eq!(bytes_needed(14), 2);
        assert_eq!(bytes_needed(15), 2);
        assert_eq!(bytes_needed(16), 2);
        assert_eq!(bytes_needed(17), 3);
    }

    #[test]
    fn pack_coils_works() {
        use crate::Coil::*;

        let single_byte = &mut [0xAA];
        let three_bytes = &mut [0xAA, 0xAA, 0xAA];

        pack_coils(&[], single_byte);
        assert_eq!(single_byte, &[0xAA]);

        pack_coils(&[On], single_byte);
        assert_eq!(single_byte, &[0b0000000_1]);

        pack_coils(&[Off], single_byte);
        assert_eq!(single_byte, &[0b0000000_0]);

        pack_coils(&[On, Off, On, Off, Off, On, Off, On], single_byte);
        assert_eq!(single_byte, &[0b10100101]);

        pack_coils(&[On, On, On, On, Off, Off, Off, Off], three_bytes);
        assert_eq!(three_bytes, &[0b00001111, 0xAA, 0xAA]);

        pack_coils(
            &[On, On, Off, Off, Off, On, Off, On, Off, Off, On],
            three_bytes,
        );
        assert_eq!(three_bytes, &[0b10100011, 0b00000_100, 0xAA]);
    }

    #[test]
    fn unpack_coils_works() {
        use crate::Coil::*;

        let single_coil = &mut [On];
        unpack_coils(&[0], single_coil);
        assert_eq!(single_coil, &[Off]);

        unpack_coils(&[1], single_coil);
        assert_eq!(single_coil, &[On]);

        let eight_coils = &mut [On, On, On, On, Off, Off, Off, Off];
        unpack_coils(&[0b01110010], eight_coils);
        assert_eq!(eight_coils, &[Off, On, Off, Off, On, On, On, Off]);

        unpack_coils(&[0b11110000], eight_coils);
        assert_eq!(eight_coils, &[Off, Off, Off, Off, On, On, On, On]);

        let twelve_coils = &mut [Off, Off, Off, Off, Off, Off, Off, Off, Off, Off, Off, Off];
        unpack_coils(&[0b10011100, 0b0000_1001], twelve_coils);
        assert_eq!(
            twelve_coils,
            &[Off, Off, On, On, On, Off, Off, On, On, Off, Off, On]
        );
    }
}
