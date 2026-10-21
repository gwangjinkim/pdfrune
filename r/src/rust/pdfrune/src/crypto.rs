use std::collections::BTreeMap;

use crate::document::Document;
use crate::error::Result;
use crate::parser::object::{PdfDict, PdfDictExt, PdfObject, PdfStream};

/// ISO 32000-1 Section 7.6.3.3 standard 32-byte password padding string.
const PDF_PASSWORD_PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// Default deterministic 16-byte document `/ID` when none is present in the trailer.
const DEFAULT_DOC_ID: [u8; 16] = *b"pdfrune-doc-id16";

/// RFC 1321 MD5 message-digest implementation in pure safe Rust.
pub fn md5(input: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;

    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut msg = input.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut m = [0u32; 16];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            m[i] = u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
        }
        let mut a = a0;
        let mut b = b0;
        let mut c = c0;
        let mut d = d0;

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            let sum = a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g]);
            b = b.wrapping_add(sum.rotate_left(S[i]));
            a = temp;
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}

/// Symmetric RC4 stream cipher implementation.
pub fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    if key.is_empty() {
        return data.to_vec();
    }
    let mut s = [0u8; 256];
    for (i, v) in s.iter_mut().enumerate() {
        *v = i as u8;
    }
    let mut j: u8 = 0;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }
    let mut i: u8 = 0;
    j = 0;
    let mut out = Vec::with_capacity(data.len());
    for &byte in data {
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[i as usize]);
        s.swap(i as usize, j as usize);
        let k = s[(s[i as usize].wrapping_add(s[j as usize])) as usize];
        out.push(byte ^ k);
    }
    out
}

fn pad_password(pw: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    let bytes = pw.as_bytes();
    let copy_len = bytes.len().min(32);
    out[..copy_len].copy_from_slice(&bytes[..copy_len]);
    if copy_len < 32 {
        out[copy_len..].copy_from_slice(&PDF_PASSWORD_PADDING[..32 - copy_len]);
    }
    out
}

/// ISO 32000-1 Algorithm 3: Computes the `/O` (Owner) 32-byte entry (Revision 3, 128-bit).
pub fn compute_owner_value(owner_pw: &str, user_pw: &str) -> [u8; 32] {
    let eff_owner = if owner_pw.is_empty() {
        user_pw
    } else {
        owner_pw
    };
    let padded_owner = pad_password(eff_owner);
    let mut hash = md5(&padded_owner);
    for _ in 0..50 {
        hash = md5(&hash);
    }
    let padded_user = pad_password(user_pw);
    let mut encrypted = rc4(&hash, &padded_user);
    for iter in 1u8..=19 {
        let mut iter_key = [0u8; 16];
        for (idx, &b) in hash.iter().enumerate() {
            iter_key[idx] = b ^ iter;
        }
        encrypted = rc4(&iter_key, &encrypted);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&encrypted[..32]);
    out
}

/// ISO 32000-1 Algorithm 2: Computes the 16-byte file encryption key (Revision 3, 128-bit).
pub fn compute_encryption_key(
    user_pw: &str,
    owner_val: &[u8],
    permissions: i32,
    doc_id: &[u8],
) -> [u8; 16] {
    let padded = pad_password(user_pw);
    let mut buf = Vec::with_capacity(84);
    buf.extend_from_slice(&padded);
    buf.extend_from_slice(&owner_val[..32.min(owner_val.len())]);
    buf.extend_from_slice(&permissions.to_le_bytes());
    buf.extend_from_slice(doc_id);
    let mut hash = md5(&buf);
    for _ in 0..50 {
        hash = md5(&hash);
    }
    hash
}

/// ISO 32000-1 Algorithm 5: Computes the `/U` (User) 32-byte entry (Revision 3, 128-bit).
pub fn compute_user_value(file_key: &[u8; 16], doc_id: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(48);
    buf.extend_from_slice(&PDF_PASSWORD_PADDING);
    buf.extend_from_slice(doc_id);
    let hash = md5(&buf);
    let mut encrypted = rc4(file_key, &hash);
    for iter in 1u8..=19 {
        let mut iter_key = [0u8; 16];
        for (idx, &b) in file_key.iter().enumerate() {
            iter_key[idx] = b ^ iter;
        }
        encrypted = rc4(&iter_key, &encrypted);
    }
    let mut out = [0u8; 32];
    out[..16].copy_from_slice(&encrypted[..16]);
    out[16..].copy_from_slice(&PDF_PASSWORD_PADDING[..16]);
    out
}

/// ISO 32000-1 Algorithm 1: Computes the per-object RC4 key for `(obj_num, gen_num)`.
pub fn compute_object_key(file_key: &[u8], obj_num: u32, gen_num: u16) -> [u8; 16] {
    let mut buf = Vec::with_capacity(file_key.len() + 5);
    buf.extend_from_slice(file_key);
    let o_le = obj_num.to_le_bytes();
    let g_le = gen_num.to_le_bytes();
    buf.extend_from_slice(&[o_le[0], o_le[1], o_le[2], g_le[0], g_le[1]]);
    md5(&buf)
}

fn transform_object_crypto(obj: &mut PdfObject, obj_key: &[u8]) {
    match obj {
        PdfObject::String(bytes) | PdfObject::HexString(bytes) => {
            *bytes = rc4(obj_key, bytes);
        }
        PdfObject::Array(arr) => {
            for item in arr {
                transform_object_crypto(item, obj_key);
            }
        }
        PdfObject::Dict(dict) => {
            for val in dict.values_mut() {
                transform_object_crypto(val, obj_key);
            }
        }
        PdfObject::Stream(stm) => {
            transform_stream_crypto(stm, obj_key);
        }
        _ => {}
    }
}

fn transform_stream_crypto(stm: &mut PdfStream, obj_key: &[u8]) {
    for val in stm.dict.values_mut() {
        transform_object_crypto(val, obj_key);
    }
    stm.data = rc4(obj_key, &stm.data);
    stm.dict
        .insert(b"Length".to_vec(), PdfObject::Int(stm.data.len() as i64));
}

/// Encrypts all objects in `objects` (except the `/Encrypt` dictionary itself) and populates
/// the `/Encrypt` and `/ID` entries in `trailer`.
pub fn encrypt_document_objects(
    objects: &mut BTreeMap<u32, PdfObject>,
    trailer: &mut PdfDict,
    user_pw: &str,
    owner_pw: &str,
) {
    let permissions: i32 = -4;
    let doc_id = DEFAULT_DOC_ID;
    let o_val = compute_owner_value(owner_pw, user_pw);
    let file_key = compute_encryption_key(user_pw, &o_val, permissions, &doc_id);
    let u_val = compute_user_value(&file_key, &doc_id);

    for (&obj_num, obj) in objects.iter_mut() {
        let obj_key = compute_object_key(&file_key, obj_num, 0);
        transform_object_crypto(obj, &obj_key);
    }

    let encrypt_obj_num = objects.keys().next_back().copied().unwrap_or(0) + 1;
    let mut enc_dict = PdfDict::new();
    enc_dict.insert(b"Filter".to_vec(), PdfObject::Name(b"Standard".to_vec()));
    enc_dict.insert(b"V".to_vec(), PdfObject::Int(2));
    enc_dict.insert(b"R".to_vec(), PdfObject::Int(3));
    enc_dict.insert(b"Length".to_vec(), PdfObject::Int(128));
    enc_dict.insert(b"P".to_vec(), PdfObject::Int(permissions as i64));
    enc_dict.insert(b"O".to_vec(), PdfObject::HexString(o_val.to_vec()));
    enc_dict.insert(b"U".to_vec(), PdfObject::HexString(u_val.to_vec()));

    objects.insert(encrypt_obj_num, PdfObject::Dict(enc_dict));
    trailer.insert(
        b"Encrypt".to_vec(),
        PdfObject::Reference((encrypt_obj_num, 0)),
    );
    trailer.insert(
        b"ID".to_vec(),
        PdfObject::Array(vec![
            PdfObject::HexString(doc_id.to_vec()),
            PdfObject::HexString(doc_id.to_vec()),
        ]),
    );
}

/// Attempts to authenticate `doc` with `password` (as either user or owner password).
/// When successful, decrypts all indirect objects in `doc.objects` in-place and returns `Ok(true)`.
pub fn authenticate_and_decrypt(doc: &mut Document, password: &str) -> Result<bool> {
    if !doc.is_encrypted || doc.is_authenticated {
        doc.is_authenticated = true;
        return Ok(true);
    }

    let enc_obj_num = doc.trailer.get_ref(b"Encrypt").map(|r| r.0);
    let Some(enc_dict) = doc
        .resolve_dict_entry(&doc.trailer, b"Encrypt")
        .and_then(PdfObject::as_dict)
        .cloned()
    else {
        return Ok(false);
    };

    // Legacy simple /UserPassword field fallback
    if let Some(expected_pw) = enc_dict.get_string_decoded(b"UserPassword") {
        if expected_pw == password {
            doc.is_authenticated = true;
            return Ok(true);
        }
        return Ok(false);
    }

    let o_bytes = enc_dict
        .get(b"O".as_slice())
        .and_then(PdfObject::as_bytes)
        .unwrap_or(&[]);
    let u_bytes = enc_dict
        .get(b"U".as_slice())
        .and_then(PdfObject::as_bytes)
        .unwrap_or(&[]);
    if o_bytes.len() < 32 || u_bytes.len() < 16 {
        return Ok(false);
    }

    let permissions = enc_dict.get_int(b"P").unwrap_or(-4) as i32;
    let doc_id: Vec<u8> = doc
        .trailer
        .get(b"ID".as_slice())
        .and_then(|o| doc.resolve(o).as_array())
        .and_then(|a| a.first())
        .and_then(PdfObject::as_bytes)
        .map(|b| b.to_vec())
        .unwrap_or_else(|| DEFAULT_DOC_ID.to_vec());

    // 1. Try `password` as user password (ISO 32000-1 Algorithm 6)
    let candidate_key = compute_encryption_key(password, o_bytes, permissions, &doc_id);
    let candidate_u = compute_user_value(&candidate_key, &doc_id);

    let matched_key = if candidate_u[..16] == u_bytes[..16] {
        Some(candidate_key)
    } else {
        // 2. Try `password` as owner password (ISO 32000-1 Algorithm 7)
        let padded_owner = pad_password(password);
        let mut hash = md5(&padded_owner);
        for _ in 0..50 {
            hash = md5(&hash);
        }
        let mut decrypted_user = o_bytes[..32].to_vec();
        for iter in (1u8..=19).rev() {
            let mut iter_key = [0u8; 16];
            for (idx, &b) in hash.iter().enumerate() {
                iter_key[idx] = b ^ iter;
            }
            decrypted_user = rc4(&iter_key, &decrypted_user);
        }
        decrypted_user = rc4(&hash, &decrypted_user);
        // Derive key using recovered padded user password
        let mut buf = Vec::with_capacity(84);
        buf.extend_from_slice(&decrypted_user[..32]);
        buf.extend_from_slice(&o_bytes[..32]);
        buf.extend_from_slice(&permissions.to_le_bytes());
        buf.extend_from_slice(&doc_id);
        let mut owner_derived_key = md5(&buf);
        for _ in 0..50 {
            owner_derived_key = md5(&owner_derived_key);
        }
        let check_u = compute_user_value(&owner_derived_key, &doc_id);
        if check_u[..16] == u_bytes[..16] {
            Some(owner_derived_key)
        } else {
            None
        }
    };

    let Some(file_key) = matched_key else {
        return Ok(false);
    };

    // Decrypt all indirect objects in-place (skipping the /Encrypt dictionary itself)
    for (&obj_num, obj) in doc.objects.iter_mut() {
        if Some(obj_num) == enc_obj_num {
            continue;
        }
        let obj_key = compute_object_key(&file_key, obj_num, 0);
        transform_object_crypto(obj, &obj_key);
    }

    doc.is_authenticated = true;
    let _ = doc.rebuild_page_tree();
    Ok(true)
}
