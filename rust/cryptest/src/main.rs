#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![no_std]
#![no_main]

use hex_literal::hex;
use k210_hal::prelude::*;
use k210_hal::stdout::Stdout;
use k210_hal::Peripherals;
use k210_shared::soc::sleep::usleep;
use k210_shared::soc::sysctl;
use k210_shared::soc::aes::{self, cipher_mode, encrypt_sel};
use k210_shared::soc::sha256::SHA256Ctx;
use riscv::asm;
use riscv_rt::entry;

struct AESTestVec {
    cipher_mode: cipher_mode,
    key: &'static [u8],
    pt: &'static [u8],
    ct: &'static [u8],
    iv: &'static [u8],
    aad: &'static [u8],
    tag: &'static [u8],
}

struct SHA256TestVec {
    data: &'static [u8],
    hash: [u8; 32],
}

#[entry]
fn main() -> ! {
    let mut p = Peripherals::take().unwrap();
    let clocks = k210_hal::clock::Clocks::new();

    // Enable clocks for AES and reset the engine
    sysctl::clock_enable(sysctl::clock::AES);
    sysctl::reset(sysctl::reset::AES);
    // Enable clocks for SHA256 and reset the engine
    sysctl::clock_enable(sysctl::clock::SHA);
    sysctl::reset(sysctl::reset::SHA);

    // Configure UART
    let serial = p
        .UARTHS
        .configure((p.pins.pin5, p.pins.pin4), 115_200.bps(), &clocks);
    let (mut tx, _) = serial.split();
    let mut stdout = Stdout(&mut tx);

    usleep(200000);
    writeln!(
        stdout,
        "Init",
    ).unwrap();

    let aes = &mut p.AES;
    let sha256 = &mut p.SHA256;

    // https://boringssl.googlesource.com/boringssl/+/2214/crypto/cipher/cipher_test.txt
    // https://github.com/plenluno/openssl/blob/master/openssl/test/evptests.txt
    // http://csrc.nist.gov/groups/ST/toolkit/BCM/documents/proposedmodes/gcm/gcm-spec.pdf
    for tv in &[
        AESTestVec {
            cipher_mode: cipher_mode::ECB,
            key: &hex!("2B7E151628AED2A6ABF7158809CF4F3C"),
            pt: &hex!("6BC1BEE22E409F96E93D7E117393172A"),
            ct: &hex!("3AD77BB40D7A3660A89ECAF32466EF97"),
            iv: &hex!(""),
            aad: &hex!(""),
            tag: &hex!(""),
        },
        AESTestVec {
            cipher_mode: cipher_mode::GCM,
            key: &hex!("e98b72a9881a84ca6b76e0f43e68647a"),
            pt: &hex!("28286a321293253c3e0aa2704a278032"),
            ct: &hex!("5a3c1cf1985dbb8bed818036fdd5ab42"),
            iv: &hex!("8b23299fde174053f3d652ba"),
            aad: &hex!(""),
            tag: &hex!("23c7ab0f952b7091cd324835043b5eb5"),
        },
        AESTestVec {
            cipher_mode: cipher_mode::GCM,
            key: &hex!("816e39070410cf2184904da03ea5075a"),
            pt: &hex!("ecafe96c67a1646744f1c891f5e69427"),
            ct: &hex!("552ebe012e7bcf90fcef712f8344e8f1"),
            iv: &hex!("32c367a3362613b27fc3e67e"),
            aad: &hex!("f2a30728ed874ee02983c294435d3c16"),
            tag: &hex!("ecaae9fc68276a45ab0ca3cb9dd9539f"),
        },
        AESTestVec {
            cipher_mode: cipher_mode::GCM,
            key: &hex!("95bcde70c094f04e3dd8259cafd88ce8"),
            pt: &hex!("32f51e837a9748838925066d69e87180f34a6437e6b396e5643b34cb2ee4f7b1"),
            ct: &hex!("8a023ba477f5b809bddcda8f55e09064d6d88aaec99c1e141212ea5b08503660"),
            iv: &hex!("12cf097ad22380432ff40a5c"),
            aad: &hex!("c783a0cca10a8d9fb8d27d69659463f2"),
            tag: &hex!("562f500dae635d60a769b466e15acd1e"),
        },
        AESTestVec {
            cipher_mode: cipher_mode::GCM,
            key: &hex!("387218b246c1a8257748b56980e50c94"),
            pt: &hex!("48f5b426baca03064554cc2b30"),
            ct: &hex!("cdba9e73eaf3d38eceb2b04a8d"),
            iv: &hex!("dd7e014198672be39f95b69d"),
            aad: &hex!(""),
            tag: &hex!("ecf90f4a47c9c626d6fb2c765d201556"),
        },
    ] {
        let mut ct_out = [0u8; 32];
        let mut tag_out = [0u8; 16];

        write!(stdout, "AES128: ").unwrap();
        aes::run(
            aes,
            tv.cipher_mode,
            encrypt_sel::ENCRYPTION,
            tv.key,
            tv.iv,
            tv.aad,
            tv.pt,
            &mut ct_out,
            &mut tag_out,
        );

        if &ct_out[0..tv.ct.len()] == tv.ct {
            write!(stdout, "MATCH").unwrap();
        } else {
            write!(stdout, "MISMATCH").unwrap();
        }

        write!(stdout, " ").unwrap();

        if tv.cipher_mode == cipher_mode::GCM {
            if &tag_out[0..tv.tag.len()] == tv.tag {
                write!(stdout, "TAGMATCH").unwrap();
            } else {
                write!(stdout, "TAGMISMATCH").unwrap();
            }
        }
        writeln!(stdout).unwrap();

    }

    // https://www.di-mgt.com.au/sha_testvectors.html
    for tv in &[
        SHA256TestVec {
            data: b"",
            hash: hex!("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        },
        SHA256TestVec {
            data: b"abc",
            hash: hex!("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        },
        SHA256TestVec {
            data: b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            hash: hex!("248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")
        },
        SHA256TestVec {
            data: b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
            hash: hex!("cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1")
        },
    ] {
        write!(stdout, "SHA256: ").unwrap();
        let mut sha = SHA256Ctx::new(sha256, tv.data.len());
        sha.update(&tv.data[..]);
        let sha_out = sha.finish();
        if sha_out == tv.hash {
            writeln!(stdout, "MATCH").unwrap();
        } else {
            writeln!(stdout, "MISMATCH").unwrap();
        }
    }

    loop {
        unsafe { asm::wfi(); }
    }
}