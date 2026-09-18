# clouddrive-decrypt

**clouddrive-decrypt** is a standalone tool designed to demo how to decrypt files encrypted by CloudDrive2.

## Usage

Decrypt an encrypted file (original behavior):

```bash
clouddrive-decrypt <input_file> <password>
```

Decrypt only an encrypted CloudDrive2 file or folder name:

```bash
clouddrive-decrypt --name "<encrypted_name>.cdcrypto" "<password>"
```

Short form:

```bash
clouddrive-decrypt -n "<encrypted_name>.cdcrypto" "<password>"
```

The name-only mode prints only the decrypted original name and does not open or decrypt file contents.
