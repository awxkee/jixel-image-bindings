/*
 * // Copyright (c) Radzivon Bartoshyk 8/2026. All rights reserved.
 * //
 * // Redistribution and use in source and binary forms, with or without modification,
 * // are permitted provided that the following conditions are met:
 * //
 * // 1.  Redistributions of source code must retain the above copyright notice, this
 * // list of conditions and the following disclaimer.
 * //
 * // 2.  Redistributions in binary form must reproduce the above copyright notice,
 * // this list of conditions and the following disclaimer in the documentation
 * // and/or other materials provided with the distribution.
 * //
 * // 3.  Neither the name of the copyright holder nor the names of its
 * // contributors may be used to endorse or promote products derived from
 * // this software without specific prior written permission.
 * //
 * // THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * // AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * // IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * // DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * // FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * // DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * // SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * // CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * // OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * // OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

pub(crate) fn has_alpha<T: Copy + PartialEq, const ALPHA_CHANNEL_INDEX: usize, const CN: usize>(
    store: &[T],
    width: usize,
    stride: usize,
    opaque: T,
) -> bool {
    assert!(ALPHA_CHANNEL_INDEX < CN);
    assert!(CN <= 4);

    let row_len = width.checked_mul(CN).expect("image row length overflow");
    assert!(stride >= row_len);

    if row_len == 0 || store.is_empty() {
        return false;
    }

    for row in store.chunks(stride) {
        let pixels = row
            .get(..row_len)
            .expect("alpha checker received a truncated image row");
        if pixels
            .as_chunks::<CN>()
            .0
            .iter()
            .any(|pixel| pixel[ALPHA_CHANNEL_INDEX] != opaque)
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::has_alpha;

    #[test]
    fn opaque_alpha_is_discardable() {
        let rgba = [1u8, 2, 3, 255, 4, 5, 6, 255];
        assert!(!has_alpha::<_, 3, 4>(&rgba, 2, 8, u8::MAX));
    }

    #[test]
    fn constant_translucent_alpha_is_not_discardable() {
        let rgba = [1u8, 2, 3, 128, 4, 5, 6, 128];
        assert!(has_alpha::<_, 3, 4>(&rgba, 2, 8, u8::MAX));
    }

    #[test]
    fn row_padding_is_ignored() {
        let rgba_with_padding = [1u16, u16::MAX, 2, 0, 3, u16::MAX, 4, 0];
        assert!(!has_alpha::<_, 1, 2>(&rgba_with_padding, 1, 4, u16::MAX));
    }
}
