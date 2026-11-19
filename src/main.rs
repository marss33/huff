mod bits;
mod huffman;

use std::env;
use std::fs;
use std::path::Path;
use std::process;

const USO: &str = "uso: huff comprimir <archivo>
     huff descomprimir <archivo.huff>";

fn main() {
    let args: Vec<String> = env::args().collect();
    let result = match args.as_slice() {
        [_, cmd, archivo] if cmd == "comprimir" => comprimir(archivo),
        [_, cmd, archivo] if cmd == "descomprimir" => descomprimir(archivo),
        _ => {
            eprintln!("{USO}");
            process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn comprimir(archivo: &str) -> Result<(), String> {
    let data = fs::read(archivo).map_err(|e| format!("no se pudo leer {archivo}: {e}"))?;
    let comprimido = huffman::compress(&data)?;
    let destino = format!("{archivo}.huff");
    fs::write(&destino, &comprimido).map_err(|e| format!("no se pudo escribir {destino}: {e}"))?;

    println!(
        "{archivo} -> {destino}: {} -> {} ({})",
        tamanio(data.len()),
        tamanio(comprimido.len()),
        ahorro(data.len(), comprimido.len())
    );
    Ok(())
}

fn descomprimir(archivo: &str) -> Result<(), String> {
    let destino = archivo
        .strip_suffix(".huff")
        .ok_or("el archivo tiene que terminar en .huff")?;
    if Path::new(destino).exists() {
        return Err(format!("{destino} ya existe, borralo o movelo antes"));
    }
    let data = fs::read(archivo).map_err(|e| format!("no se pudo leer {archivo}: {e}"))?;
    let original = huffman::decompress(&data)?;
    fs::write(destino, &original).map_err(|e| format!("no se pudo escribir {destino}: {e}"))?;

    println!("{archivo} -> {destino}: {}", tamanio(original.len()));
    Ok(())
}

fn tamanio(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn ahorro(antes: usize, despues: usize) -> String {
    if antes == 0 {
        return "archivo vacio".to_string();
    }
    let porcentaje = (1.0 - despues as f64 / antes as f64) * 100.0;
    if porcentaje >= 0.0 {
        format!("ahorro {porcentaje:.1}%")
    } else {
        format!("crecio {:.1}%", -porcentaje)
    }
}
