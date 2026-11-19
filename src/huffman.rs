use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::bits::{BitReader, BitWriter};

// formato del archivo comprimido:
//   "HUFF"
//   largo original (u64)
//   cantidad de bytes distintos (u16)
//   por cada uno: el byte (u8) y cuantas veces aparece (u32)
//   los codigos de todo el archivo, uno atras del otro
// con las frecuencias se vuelve a armar el mismo arbol para descomprimir

const MAGIC: &[u8; 4] = b"HUFF";

enum Node {
    Leaf(u8),
    Internal(usize, usize), // izquierda (bit 0) y derecha (bit 1)
}

// los nodos van en un vector y la raiz es el ultimo
fn build_tree(freqs: &[u32; 256]) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut heap = BinaryHeap::new();

    for (byte, &freq) in freqs.iter().enumerate() {
        if freq > 0 {
            heap.push(Reverse((freq as u64, nodes.len())));
            nodes.push(Node::Leaf(byte as u8));
        }
    }

    // se juntan siempre los dos de menor frecuencia. el indice desempata,
    // asi al descomprimir sale exactamente el mismo arbol
    while heap.len() > 1 {
        let Reverse((f1, a)) = heap.pop().unwrap();
        let Reverse((f2, b)) = heap.pop().unwrap();
        heap.push(Reverse((f1 + f2, nodes.len())));
        nodes.push(Node::Internal(a, b));
    }
    nodes
}

fn codes(nodes: &[Node]) -> Vec<Vec<bool>> {
    let mut codes = vec![Vec::new(); 256];
    let Some(root) = nodes.len().checked_sub(1) else {
        return codes;
    };
    if let Node::Leaf(byte) = nodes[root] {
        // un solo byte distinto: igual hace falta un bit por byte
        codes[byte as usize] = vec![false];
        return codes;
    }
    fill_codes(nodes, root, &mut Vec::new(), &mut codes);
    codes
}

fn fill_codes(nodes: &[Node], node: usize, path: &mut Vec<bool>, codes: &mut [Vec<bool>]) {
    match nodes[node] {
        Node::Leaf(byte) => codes[byte as usize] = path.clone(),
        Node::Internal(left, right) => {
            path.push(false);
            fill_codes(nodes, left, path, codes);
            path.pop();
            path.push(true);
            fill_codes(nodes, right, path, codes);
            path.pop();
        }
    }
}

pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut freqs = [0u32; 256];
    for &byte in data {
        let f = &mut freqs[byte as usize];
        *f = f.checked_add(1).ok_or("el archivo es demasiado grande")?;
    }

    let mut out = MAGIC.to_vec();
    out.extend((data.len() as u64).to_le_bytes());
    let used: Vec<usize> = (0..256).filter(|&b| freqs[b] > 0).collect();
    out.extend((used.len() as u16).to_le_bytes());
    for &b in &used {
        out.push(b as u8);
        out.extend(freqs[b].to_le_bytes());
    }

    let codes = codes(&build_tree(&freqs));
    let mut bits = BitWriter::new();
    for &byte in data {
        for &bit in &codes[byte as usize] {
            bits.push(bit);
        }
    }
    out.extend(bits.finish());
    Ok(out)
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let cortado = || "el archivo esta cortado o no es de huff".to_string();

    if data.len() < 14 || &data[..4] != MAGIC {
        return Err("no es un archivo comprimido con huff".to_string());
    }
    let len = u64::from_le_bytes(data[4..12].try_into().unwrap()) as usize;
    let count = u16::from_le_bytes(data[12..14].try_into().unwrap()) as usize;

    let mut freqs = [0u32; 256];
    let mut pos = 14;
    for _ in 0..count {
        let entry = data.get(pos..pos + 5).ok_or_else(cortado)?;
        freqs[entry[0] as usize] = u32::from_le_bytes(entry[1..5].try_into().unwrap());
        pos += 5;
    }
    if freqs.iter().map(|&f| f as u64).sum::<u64>() != len as u64 {
        return Err(cortado());
    }

    let nodes = build_tree(&freqs);
    let Some(root) = nodes.len().checked_sub(1) else {
        return Ok(Vec::new());
    };
    if let Node::Leaf(byte) = nodes[root] {
        return Ok(vec![byte; len]);
    }

    let mut out = Vec::with_capacity(len);
    let mut bits = BitReader::new(&data[pos..]);
    while out.len() < len {
        let mut node = root;
        while let Node::Internal(left, right) = nodes[node] {
            node = if bits.next().ok_or_else(cortado)? {
                right
            } else {
                left
            };
        }
        if let Node::Leaf(byte) = nodes[node] {
            out.push(byte);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ida_y_vuelta(data: &[u8]) -> Vec<u8> {
        let comprimido = compress(data).unwrap();
        assert_eq!(decompress(&comprimido).unwrap(), data);
        comprimido
    }

    #[test]
    fn texto() {
        let texto =
            "el algoritmo de huffman usa codigos mas cortos para las letras que mas aparecen. "
                .repeat(50);
        let comprimido = ida_y_vuelta(texto.as_bytes());
        assert!(comprimido.len() < texto.len() * 6 / 10);
    }

    #[test]
    fn casos_borde() {
        ida_y_vuelta(b"");
        ida_y_vuelta(b"a");
        ida_y_vuelta(b"aaaaaaaaaaaaaaaaaaaa");
        ida_y_vuelta(b"ab");
        let todos: Vec<u8> = (0..=255).collect();
        ida_y_vuelta(&todos);
    }

    #[test]
    fn bytes_al_azar() {
        // generador simple para no depender de ningun crate
        let mut x: u32 = 12345;
        let data: Vec<u8> = (0..100_000)
            .map(|_| {
                x = x.wrapping_mul(1103515245).wrapping_add(12345);
                (x >> 16) as u8
            })
            .collect();
        ida_y_vuelta(&data);
    }

    #[test]
    fn los_codigos_no_son_prefijo_de_otros() {
        let mut freqs = [0u32; 256];
        for (i, f) in freqs.iter_mut().enumerate().take(40) {
            *f = (i as u32 % 7 + 1) * 3;
        }
        let codes = codes(&build_tree(&freqs));
        let usados: Vec<&Vec<bool>> = codes.iter().filter(|c| !c.is_empty()).collect();
        assert_eq!(usados.len(), 40);
        for a in &usados {
            for b in &usados {
                if a != b {
                    assert!(!b.starts_with(a));
                }
            }
        }
    }

    #[test]
    fn archivos_invalidos() {
        assert!(decompress(b"hola").is_err());
        assert!(decompress(b"no es huff para nada").is_err());
        let comprimido = compress(b"un texto cualquiera").unwrap();
        assert!(decompress(&comprimido[..comprimido.len() - 2]).is_err());
    }
}
