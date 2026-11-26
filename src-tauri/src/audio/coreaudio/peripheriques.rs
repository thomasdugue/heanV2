//! Les périphériques de sortie : les trouver, et lire leurs propriétés.
//!
//! Porté de `coreaudio_backend.rs` de V1 — `get_all_output_devices`,
//! `device_has_output_streams`, `get_device_name`, `get_max_channels`,
//! `get_device_transport_type` — sans changement de comportement. Les noms restent
//! reconnaissables, en français comme le reste du code de V2.

use coreaudio_sys::{
    kAudioDevicePropertyScopeOutput, kAudioDevicePropertyStreamConfiguration,
    kAudioDevicePropertyTransportType, kAudioHardwarePropertyDefaultOutputDevice,
    kAudioHardwarePropertyDevices, kAudioObjectPropertyName, kAudioObjectPropertyScopeInput,
    kAudioObjectUnknown, AudioBufferList, AudioObjectID,
};
use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};

use super::hal;
use crate::audio::error::ErreurAudio;

/// `'airp'`. Un périphérique AirPlay se reconnaît par là, et c'est ce qui déclenche toute la
/// stratégie de PLAT-1 (🔴).
pub const TRANSPORT_AIRPLAY: u32 = 0x6169_7270;
/// `'blue'`. Le Bluetooth partage avec AirPlay la propriété de disparaître quand il est
/// inactif, mais pas l'incompatibilité avec le mode exclusif.
pub const TRANSPORT_BLUETOOTH: u32 = 0x626C_7565;

/// Tout ce qu'on sait d'un périphérique de sortie, au moment où le cache a été rafraîchi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoPeripherique {
    pub id: AudioObjectID,
    pub nom: String,
    pub par_defaut: bool,
    /// Les fréquences standard que le périphérique annonce savoir servir, triées.
    pub frequences: Vec<u32>,
    pub frequence_actuelle: u32,
    pub canaux_max: u16,
    pub transport: u32,
}

impl InfoPeripherique {
    pub fn est_airplay(&self) -> bool {
        self.transport == TRANSPORT_AIRPLAY
    }

    pub fn est_bluetooth(&self) -> bool {
        self.transport == TRANSPORT_BLUETOOTH
    }

    /// Les fréquences, écrites pour un humain : « 44 100, 48 000, 96 000 Hz ».
    /// Sert le message d'`ErreurAudio::FrequenceNonServie`, que Thomas lit.
    pub fn frequences_en_clair(&self) -> String {
        if self.frequences.is_empty() {
            return "aucune fréquence annoncée".to_owned();
        }
        let liste: Vec<String> = self.frequences.iter().map(|f| f.to_string()).collect();
        format!("{} Hz", liste.join(", "))
    }
}

/// Le périphérique de sortie par défaut du système.
pub fn peripherique_par_defaut() -> Result<AudioObjectID, ErreurAudio> {
    let adr = hal::globale(kAudioHardwarePropertyDefaultOutputDevice);
    let id = hal::lire(
        hal::SYSTEME,
        &adr,
        kAudioObjectUnknown as AudioObjectID,
        "lire le périphérique de sortie par défaut",
    )?;
    if id == kAudioObjectUnknown as AudioObjectID {
        return Err(ErreurAudio::AucunPeripherique);
    }
    Ok(id)
}

/// Change le périphérique de sortie par défaut **du système entier**.
///
/// ⚠️ **PLAT-1 (🔴)** : sur un périphérique AirPlay périmé, cet appel est accepté sans rien
/// faire. Il ne prouve donc jamais à lui seul que le routage a changé. Il n'est appelé que
/// pour *activer* un AirPlay (le seul moyen qu'il existe dans le HAL) ou pour synchroniser
/// le volume système entre deux périphériques filaires — jamais pour quitter un AirPlay, ce
/// qui tuerait sa session.
pub fn definir_peripherique_par_defaut_systeme(id: AudioObjectID) -> Result<(), ErreurAudio> {
    let adr = hal::globale(kAudioHardwarePropertyDefaultOutputDevice);
    hal::ecrire(
        hal::SYSTEME,
        &adr,
        &id,
        "définir le périphérique par défaut du système",
    )
}

/// Tous les périphériques capables de sortir du son.
///
/// Le filtre est celui de V1, et chacune de ses trois branches a une raison :
///   1. des flux de sortie → c'est une sortie, point ;
///   2. AirPlay sans flux → un AirPlay inactif n'ouvre ses flux qu'une fois choisi comme
///      défaut système, donc l'exclure le rendrait invisible exactement quand on veut le
///      choisir ;
///   3. Bluetooth sans flux de sortie **mais sans flux d'entrée non plus** → un casque
///      Bluetooth inactif. Le test d'entrée est ce qui écarte les profils micro (HFP/SCO),
///      qui sont des entrées déguisées en périphériques.
pub fn tous_les_peripheriques_de_sortie() -> Result<Vec<AudioObjectID>, ErreurAudio> {
    let adr = hal::globale(kAudioHardwarePropertyDevices);
    let tous: Vec<AudioObjectID> =
        hal::lire_tableau(hal::SYSTEME, &adr, "lister les périphériques")?;
    Ok(tous
        .into_iter()
        .filter(|&id| {
            if a_des_flux_de_sortie(id) {
                return true;
            }
            match type_de_transport(id) {
                TRANSPORT_AIRPLAY => true,
                TRANSPORT_BLUETOOTH => !a_des_flux_dentree(id),
                _ => false,
            }
        })
        .collect())
}

/// Le nombre de tampons annoncés par la configuration de flux, dans une portée donnée.
/// Zéro si la propriété est illisible : pour un périphérique qui vient de disparaître, « je
/// ne sais pas » et « aucun flux » mènent à la même conclusion.
fn nombre_de_tampons(id: AudioObjectID, portee: u32) -> u32 {
    let adr = hal::adresse(kAudioDevicePropertyStreamConfiguration, portee);
    let Ok(octets) = hal::lire_octets(id, &adr, "lire la configuration de flux") else {
        return 0;
    };
    if octets.len() < std::mem::size_of::<AudioBufferList>() {
        return 0;
    }
    // SÛRETÉ : le tampon vient du HAL pour cette propriété exacte et fait au moins la taille
    // d'un `AudioBufferList` ; on ne lit que son premier champ.
    unsafe { (*(octets.as_ptr() as *const AudioBufferList)).mNumberBuffers }
}

pub fn a_des_flux_de_sortie(id: AudioObjectID) -> bool {
    nombre_de_tampons(id, kAudioDevicePropertyScopeOutput) > 0
}

pub fn a_des_flux_dentree(id: AudioObjectID) -> bool {
    nombre_de_tampons(id, kAudioObjectPropertyScopeInput) > 0
}

/// Le nom du périphérique, tel que macOS l'affiche.
pub fn nom_du_peripherique(id: AudioObjectID) -> Result<String, ErreurAudio> {
    let adr = hal::globale(kAudioObjectPropertyName);
    let brut: CFStringRef = hal::lire(id, &adr, std::ptr::null(), "lire le nom du périphérique")?;
    if brut.is_null() {
        return Err(ErreurAudio::CoreAudio {
            operation: "lire le nom du périphérique".to_owned(),
            code: 0,
        });
    }
    // SÛRETÉ : le HAL rend une `CFStringRef` valide suivant la *get rule* — on ne possède
    // pas la référence, donc `wrap_under_get_rule` (qui la retient) est la bonne enveloppe.
    // `wrap_under_create_rule` la libérerait à tort et provoquerait un double relâchement.
    let texte = unsafe { CFString::wrap_under_get_rule(brut) };
    Ok(texte.to_string())
}

/// Le nombre de canaux de sortie, tous tampons confondus. **2 si la propriété est
/// illisible** : c'est le repli de V1, et il est tenable ici parce que le nombre de canaux
/// de la *lecture* vient du fichier, pas de cette valeur — elle ne sert qu'à l'affichage.
pub fn canaux_max(id: AudioObjectID) -> u16 {
    let adr = hal::adresse(
        kAudioDevicePropertyStreamConfiguration,
        kAudioDevicePropertyScopeOutput,
    );
    let Ok(octets) = hal::lire_octets(id, &adr, "lire la configuration de flux") else {
        return 2;
    };
    if octets.len() < std::mem::size_of::<AudioBufferList>() {
        return 2;
    }
    // SÛRETÉ : même propriété et même garantie de taille que `nombre_de_tampons` ; le
    // nombre de tampons est relu ici pour borner le parcours à ce que le HAL a écrit.
    let total: u32 = unsafe {
        let liste = octets.as_ptr() as *const AudioBufferList;
        let combien = (*liste).mNumberBuffers as usize;
        let disponibles = (octets.len() - std::mem::size_of::<AudioBufferList>())
            / std::mem::size_of::<coreaudio_sys::AudioBuffer>()
            + 1;
        std::slice::from_raw_parts((*liste).mBuffers.as_ptr(), combien.min(disponibles))
            .iter()
            .map(|t| t.mNumberChannels)
            .sum()
    };
    u16::try_from(total).unwrap_or(2)
}

/// Le type de transport (`'airp'`, `'blue'`, `'usb '`…). **Zéro si illisible**, c'est-à-dire
/// « pas AirPlay » — le repli le plus prudent, puisqu'il n'active aucun traitement
/// particulier. V1 ignorait purement le code de retour ici ; le résultat est le même, mais
/// désormais l'échec passe par le chemin des erreurs et peut être journalisé.
pub fn type_de_transport(id: AudioObjectID) -> u32 {
    let adr = hal::globale(kAudioDevicePropertyTransportType);
    hal::lire(id, &adr, 0u32, "lire le type de transport").unwrap_or(0)
}
