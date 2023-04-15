use anyhow::{anyhow, Ok};
use std::{
    cell::RefCell,
    fmt,
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use log::trace;

use crate::{aarch64, asm_helper::asm_to_be_bytes, hook::*, LINE_ENDLING};

pub struct Patcher {
    image: RefCell<File>,
    image_path: PathBuf,
    patches: Vec<PatchInfo>,
}

struct PatchInfo {
    offset: usize,
    bytes: Vec<u8>,
    check: bool,
}

impl fmt::Debug for PatchInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PatchInfo")
            .field("offset", &self.offset)
            .field("bytes_size", &self.bytes.len())
            .finish()
    }
}

impl Patcher {
    pub fn new(image_path: &str) -> anyhow::Result<Self> {
        let image_path = PathBuf::from(image_path);
        let image_file = File::options().read(true).write(true).open(&image_path)?;
        trace!(
            "image_path: {}, file metadata: {:#?}",
            image_path.to_string_lossy(),
            image_file.metadata()?
        );
        Ok(Self {
            image: RefCell::new(image_file),
            image_path,
            patches: vec![],
        })
    }

    pub fn patch_root_key(&mut self, root_key: &str, offset: usize) -> usize {
        trace!("> patch_root_key");
        let bytes = root_key.as_bytes();
        let root_key_size = bytes.len();
        self.add_patch(offset, bytes.to_vec(), true);
        
        offset + root_key_size
    }

    pub fn patch_do_execve(&mut self, hook: DoExecveHook) -> anyhow::Result<usize> {
        trace!("> patch_do_execve hook: {:#?}", hook);
        let jump_back_addr = hook.hookee_entry + 4;
        let jump_back_relative_offset = jump_back_addr as i64 - (hook.hooker_entry as i64 + 39 * 4);
        let hooker_asm_text = aarch64! {
            "MOV X0, X0";                                   // Reserved for hookee's first instruction
            "STP X7, X8, [sp, #-16]!";
            "STP X9, X10, [sp, #-16]!";
            "STP X11, X12, [sp, #-16]!";
            "MOV X7, 0xFFFFFFFFFFFFF001";                   // X7 = (unsigned long)(-MAX_ERRNO)
            "CMP X1, X7";                                   // compare X1 and X7
            "BCS #120";                                     // if X1 > X7 goto end
            "LDR X7, [X1]";                                 // X7 = *X1
            "CBZ X7, #112";                                 // if X7 == 0 goto end
            "ADR X8, #-84";                                 // X8 = &root_key
            "MOV X9, #0";                                   // X9 = 0
            "LDRB W10, [X7, X9]";                           // W10 = *(X7 + X9)	#label1
            "CBZ W10, #96";                                 // if W10 == 0 goto end
            "LDRB W11, [X8, X9]";                           // W11 = *(X8 + X9)
            "CBZ W11, #88";                                 // if W11 == 0 goto end
            "CMP W10, W11";                                 // compare W10 and W11
            "B.NE #80";                                     // if W10 != W11 goto end
            "ADD X9, X9, 1";                                // X9 += 1
            "CMP X9, #{}", hook.root_key_size;              // compare X9 and strlen(root_key)
            "BLT #-32";                                     // if X9 < strlen(root_key) goto #label1
            "MRS X8, SP_EL0";                               // X8 = (struct task_struct *) current_thread_info()
            "LDR X10, [X8, #{}]", hook.cred_offset;         // X10 = X8->cred
            "MOV X7, #4";                                   // X7 = 4
            "MOV W9, WZR";                                  // W9 = 0
            "STR W9, [X10, X7]";                            // *(X10 + X7) = W9	#label2
            "ADD X7, X7, 4";                                // X7 += 4
            "CMP X7, #40";                                  // compare X7 and 40
            "BLT #-12";                                     // if X7 < 40 goto label2
            "MOV W9, 0xFFFFFFFF";                           // W9 = 0xFFFFFFFF
            "CMP X7, #80";                                  // compare X7 and 80
            "BLT #-24";                                     // if X7 < 80 goto label2
            "LDXR W10, [X8]";                               // W10 = *X8
            "BIC W10, W10,#0xFFF";                          // X10 = X10 & ~(0xFFF)
            "STXR W11, W10, [X8]";                          // *X8 = W10
            "STR WZR, [X8, #{}]", hook.seccomp_offset;      // X8->seccomp.mode = 0
            "STR XZR, [X8, #{}]", hook.seccomp_offset + 8;  // X8->seccomp.filter = 0
            "LDP X11, X12, [sp], #16";                      // end
            "LDP X9, X10, [sp], #16";
            "LDP X7, X8, [sp], #16";
            "B #{}", jump_back_relative_offset;
        };

        let hooker_size = self.add_hooker_patch(&hook, 0, asm_to_be_bytes(&hooker_asm_text)?)?;
        self.add_hookee_patch(&hook)?;
        let next_offset = hook.hooker_entry + hooker_size;
        Ok(next_offset)
    }

    pub fn patch_avc_denied(&mut self, hook: AVCDeniedHook) -> anyhow::Result<usize> {
        trace!("> patch_avc_denied hook: {:#?}", hook);
        let jump_back_addr = hook.hookee_entry + 4;
        let jump_back_relative_offset = jump_back_addr as i64 - (hook.hooker_entry as i64 + 28 * 4);
        let hooker_asm_text = aarch64! {
            "STP X9, X10, [sp, #-16]!";
            "STP X7, X8, [sp, #-16]!";
            "MRS X7, SP_EL0";			            // X7 = (struct task_struct *) current_thread_info()
            "LDR X7, [X7, #{}]", hook.cred_offset;  // X7 = X7->cred
            "CBZ X7, #84";			                // if X7 == 0 goto end1
            "MOV X8, #4";				            // X8 = 4
            "MOV W9, WZR";			                // W9 = 0
            "LDR W10, [X7, X8]";		            // W10 = *(X7 + X8)	#label1
            "CMP W10, W9";			                // compare W10 and W9
            "B.NE #64"; 				            // if W10 != W9 goto end1
            "ADD X8, X8, 4";			            // X8 += 4
            "CMP X8, #36";			                // compare X8 and 36
            "BLT #-20";				                // if X8 < 36 goto label1
            "ADD X8, X8, 12";			            // X8 += 12
            "MOV X9, 0x3FFFFFFFFF";	                // X9 = 0x3FFFFFFFFF
            "LDR X10, [X7, X8]";		            // X10 = *(X7 + X8)	#label2
            "ADD X8, X8, 8";			            // X8 += 8
            "CMP X10, X9";			                // compare X10 and X9
            "B.CC #28";				                // if X10 < X9 goto end1
            "CMP X8, #72";			                // compare X8 and 72
            "BLT #-20";				                // if X8 < 72 goto label2
            "LDP X9, X10, [sp], #16";	            // end2
            "LDP X7, X8, [sp], #16";
            "MOV W0, WZR";
            "RET";
            "LDP X9, X10, [sp], #16";               // end1
            "LDP X7, X8, [sp], #16";
            "MOV X0, X0"; 			                // Reserved for hookee's first instruction
            "B #{}", jump_back_relative_offset;
        };

        let hooker_size =
            self.add_hooker_patch(&hook, 27 * 4, asm_to_be_bytes(&hooker_asm_text)?)?;
        self.add_hookee_patch(&hook)?;
        let next_offset = hook.hooker_entry + hooker_size;
        Ok(next_offset)
    }

    pub fn apply_patches(&mut self) -> anyhow::Result<()> {
        self.backup_image()?;
        for patch in &self.patches {
            trace!("patch: {:#?}", patch);
            self.write_bytes(patch.offset, &patch.bytes, patch.check)?;
        }
        self.flush()?;
        Ok(())
    }

    fn add_hooker_patch(
        &mut self,
        hook: &impl Hook,
        reserved_start: usize,
        mut hooker_bytes: Vec<u8>,
    ) -> anyhow::Result<usize> {
        let backup_hookee_entry_bytes = self.read_bytes(hook.hookee_entry(), 4)?;
        trace!(
            "backup_hookee_entry_bytes: {:?}",
            &backup_hookee_entry_bytes
        );
        hooker_bytes.splice(
            reserved_start..(reserved_start + 4),
            backup_hookee_entry_bytes,
        );
        let hooker_size = hooker_bytes.len();
        self.add_patch(hook.hooker_entry(), hooker_bytes, true);
        Ok(hooker_size)
    }

    fn add_hookee_patch(&mut self, hook: &impl Hook) -> anyhow::Result<()> {
        let jump_to_hooker_relative_offset =
            hook.hooker_entry() as i64 - hook.hookee_entry() as i64;
        let jump_to_hooker_asm_text = aarch64!("B #{}", jump_to_hooker_relative_offset);
        let jump_to_hooker_be_bytes = asm_to_be_bytes(&jump_to_hooker_asm_text)?;
        self.add_patch(hook.hookee_entry(), jump_to_hooker_be_bytes, false);
        Ok(())
    }

    fn add_patch(&mut self, offset: usize, bytes: Vec<u8>, check: bool) {
        self.patches.push(PatchInfo {
            offset,
            bytes,
            check,
        })
    }

    fn backup_image(&self) -> anyhow::Result<()> {
        let image_file_name = self.image_path.file_name().unwrap().to_string_lossy();
        let image_parent_dir = self.image_path.parent().unwrap();
        let backup_file_name = image_file_name + ".bak";
        let backup_image_path = image_parent_dir.join(backup_file_name.to_string());
        if !backup_image_path.exists() {
            println!(
                "Backup image: {} -> {}",
                &self.image_path.to_string_lossy(),
                &backup_image_path.to_string_lossy()
            );
            fs::copy(&self.image_path, backup_image_path)?;
        }
        Ok(())
    }

    fn read_bytes(&self, offset: usize, size: usize) -> anyhow::Result<Vec<u8>> {
        let mut image = self.image.borrow_mut();
        image.seek(SeekFrom::Start(offset as u64))?;
        let mut buf = vec![0u8; size];
        image.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn write_bytes(&self, offset: usize, buf: &[u8], check: bool) -> anyhow::Result<usize> {
        fn ensure_all_zero(image: &mut File, size: usize) -> anyhow::Result<()> {
            let bak_pos = image.stream_position()?;
            let mut tmp = vec![0u8; size];
            image.read_exact(&mut tmp)?;
            if tmp.iter().any(|&b| b != 0) {
                return Err(anyhow!("Image offset: {} has non-zero data", bak_pos));
            }
            image.seek(SeekFrom::Start(bak_pos))?;
            Ok(())
        }

        let image = &mut self.image.borrow_mut();
        image.seek(SeekFrom::Start(offset as u64))?;
        if check {
            let buf_size = buf.len();
            ensure_all_zero(image, buf_size)?;
        }
        trace!("write offset: {}, size: {}", offset, buf.len());
        Ok(image.write(buf)?)
    }

    fn flush(&self) -> anyhow::Result<()> {
        Ok(self.image.borrow_mut().flush()?)
    }
}
