//! Le seul endroit qui connaisse la convention d'échelle de symphonia.
//!
//! # La convention, mesurée et non supposée
//!
//! Symphonia remplit un `AudioBuffer<T>` dont les valeurs occupent **toute la plage de
//! `T`**, et non la plage de la profondeur du fichier. Conséquence, relevée sur les
//! fixtures de ce dépôt :
//!
//! | source | tampon rendu | contenu |
//! |---|---|---|
//! | FLAC 16 bits | `S32` | l'échantillon natif décalé de 16 bits |
//! | FLAC 24 bits | `S32` | l'échantillon natif décalé de 8 bits |
//! | WAV / AIFF 16 bits | `S16` | l'échantillon natif, **non** décalé |
//! | ALAC | `S32` | décalé, et le conteneur ne déclare pas la profondeur |
//! | MP3 / AAC / Vorbis | `F32` | flottant dans \[-1, 1\] |
//!
//! Ça tombe exactement sur notre format interne : un tampon `S32` est **déjà** à l'échelle
//! 32 bits, donc le chemin FLAC — celui du bit-perfect — ne fait aucune conversion.
//!
//! Cette convention n'est écrite nulle part dans un contrat stable : c'est un détail
//! d'implémentation d'une bibliothèque tierce. Une mise à jour pourrait la changer, et le
//! son serait alors 65 536 fois trop faible **sans qu'aucun test de lecture n'échoue**.
//! D'où `tests::la_convention_dechelle_de_symphonia_est_celle_quon_croit`, qui échoue avec
//! un message explicite plutôt que de laisser le test bit-perfect annoncer « les octets
//! diffèrent » sans dire pourquoi.

use symphonia::core::audio::{AudioBufferRef, Signal};

use super::samples::depuis_flottant;

/// Ajoute à `sortie` les échantillons de `tampon`, **entrelacés**, en `i32` mis à l'échelle
/// sur 32 bits.
pub fn ajouter_entrelace(tampon: &AudioBufferRef<'_>, sortie: &mut Vec<i32>) {
    let trames = tampon.frames();
    let canaux = tampon.spec().channels.count();
    sortie.reserve(trames * canaux);

    // Une macro plutôt qu'une fonction générique : les variantes de `AudioBufferRef` ne
    // partagent pas de trait commun exposant `chan()`, donc chaque bras doit être écrit
    // avec son type concret. La macro évite de recopier dix fois la double boucle.
    macro_rules! entrelacer {
        ($b:expr, $conv:expr) => {{
            let b = $b;
            let f: fn(_) -> i32 = $conv;
            for t in 0..trames {
                for c in 0..canaux {
                    sortie.push(f(b.chan(c)[t]));
                }
            }
        }};
    }

    match tampon {
        // Déjà à l'échelle 32 bits : le chemin du bit-perfect ne touche à rien.
        AudioBufferRef::S32(b) => entrelacer!(b, |v: i32| v),
        AudioBufferRef::S24(b) => {
            entrelacer!(b, |v: symphonia::core::sample::i24| v.inner() << 8)
        }
        AudioBufferRef::S16(b) => entrelacer!(b, |v: i16| i32::from(v) << 16),
        AudioBufferRef::S8(b) => entrelacer!(b, |v: i8| i32::from(v) << 24),
        // Les variantes non signées sont centrées : la valeur médiane devient le zéro.
        AudioBufferRef::U32(b) => {
            entrelacer!(b, |v: u32| (v as i64 - i64::from(u32::MAX) / 2 - 1) as i32)
        }
        AudioBufferRef::U24(b) => {
            entrelacer!(b, |v: symphonia::core::sample::u24| (v.inner() as i32
                - (1 << 23))
                << 8)
        }
        AudioBufferRef::U16(b) => {
            entrelacer!(b, |v: u16| (i32::from(v) - (1 << 15)) << 16)
        }
        AudioBufferRef::U8(b) => entrelacer!(b, |v: u8| (i32::from(v) - (1 << 7)) << 24),
        AudioBufferRef::F32(b) => entrelacer!(b, |v: f32| depuis_flottant(v)),
        AudioBufferRef::F64(b) => entrelacer!(b, |v: f64| depuis_flottant(v as f32)),
    }
}

#[cfg(test)]
mod tests {
    use symphonia::core::audio::{AsAudioBufferRef, AudioBuffer, SignalSpec};
    use symphonia::core::sample::i24;

    use super::*;

    fn spec() -> SignalSpec {
        SignalSpec::new(44_100, symphonia::core::audio::Channels::FRONT_LEFT)
    }

    #[test]
    fn un_tampon_s16_est_decale_de_16_bits() {
        let mut b = AudioBuffer::<i16>::new(3, spec());
        b.render_silence(Some(3));
        b.chan_mut(0).copy_from_slice(&[0, 405, -316]);
        let mut out = Vec::new();
        ajouter_entrelace(&b.as_audio_buffer_ref(), &mut out);
        assert_eq!(out, vec![0, 405 << 16, -316 << 16]);
    }

    #[test]
    fn un_tampon_s32_passe_tel_quel() {
        let mut b = AudioBuffer::<i32>::new(2, spec());
        b.render_silence(Some(2));
        b.chan_mut(0).copy_from_slice(&[26_542_080, -20_709_376]);
        let mut out = Vec::new();
        ajouter_entrelace(&b.as_audio_buffer_ref(), &mut out);
        assert_eq!(out, vec![26_542_080, -20_709_376], "aucune conversion");
    }

    #[test]
    fn un_tampon_s24_est_decale_de_8_bits() {
        let mut b = AudioBuffer::<i24>::new(2, spec());
        b.render_silence(Some(2));
        b.chan_mut(0)
            .copy_from_slice(&[i24::from(23_833), i24::from(-80_755)]);
        let mut out = Vec::new();
        ajouter_entrelace(&b.as_audio_buffer_ref(), &mut out);
        assert_eq!(out, vec![23_833 << 8, -80_755 << 8]);
    }

    #[test]
    fn un_tampon_non_signe_est_recentre_sur_zero() {
        let mut b = AudioBuffer::<u16>::new(3, spec());
        b.render_silence(Some(3));
        // La médiane d'un u16 est le silence ; les extrêmes sont les bornes signées.
        b.chan_mut(0).copy_from_slice(&[32_768, 0, 65_535]);
        let mut out = Vec::new();
        ajouter_entrelace(&b.as_audio_buffer_ref(), &mut out);
        assert_eq!(out[0], 0, "la médiane doit être le silence");
        assert!(out[1] < 0, "le minimum doit être négatif");
        assert!(out[2] > 0, "le maximum doit être positif");
    }

    #[test]
    fn lentrelacement_alterne_bien_les_canaux() {
        let s = SignalSpec::new(
            44_100,
            symphonia::core::audio::Channels::FRONT_LEFT
                | symphonia::core::audio::Channels::FRONT_RIGHT,
        );
        let mut b = AudioBuffer::<i16>::new(2, s);
        b.render_silence(Some(2));
        b.chan_mut(0).copy_from_slice(&[1, 3]);
        b.chan_mut(1).copy_from_slice(&[2, 4]);
        let mut out = Vec::new();
        ajouter_entrelace(&b.as_audio_buffer_ref(), &mut out);
        // L, R, L, R — et non L, L, R, R : une inversion ici rendrait le son méconnaissable.
        assert_eq!(out, vec![1 << 16, 2 << 16, 3 << 16, 4 << 16]);
    }
}
