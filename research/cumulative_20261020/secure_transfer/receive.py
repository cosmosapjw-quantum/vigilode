"""Import four source ZIPs without publishing temporary authenticated URLs.

The one-use RSA private key remains in RUNNER_TEMP. Only public keys, encrypted
requests, original user-approved artifacts and hash receipts enter the repo.
No scientific scripts from the archives are executed.
"""
import base64, hashlib, io, json, os, stat, subprocess, sys, tempfile, time
import urllib.parse, urllib.request, zipfile
from pathlib import Path, PurePosixPath

NODE = Path('research/cumulative_20261020')
MEMBER_INDEX = '16c0648d6bdeaa06e1622ed55473bf3b60f0af5679aeb04aa96d3f59ad6ddd57'
EXPECTED = [
    ('integration','VigilODE_Integration_Research_20261020.zip',47810,'54f4f4a99d552388fc28d01f5f367176e301c2af15c3cd8a9c7a30b4e5dfd4f1',22,'1KFeUnjwSxuVF8NXujyLx9QOr69hbVM6-'),
    ('derivative_free','VigilODE_DerivativeFree_FVM_Research_20261020.zip',121653,'a17864dd8fa1951c7ddcd236740c564c10b323248b1bb30c92eea32bc1e745e7',18,'1kiZpLR8RAIMmF5x_j_siQ-vJdJMtvcDh'),
    ('legacy_vode_comparison','VigilODE_VODE_Diffsol_Comparison_20261019.zip',18869,'790b4621e09dff21e196cc548b7851ca751d09f7b0287c52e0bc857fca2ad9ae',11,'1fPfXkgTjKYzelxufhz56XehfLZ4xKlZF'),
    ('hybrid_recovery','VigilODE_Hybrid_AD_Recovery_20261020.zip',66374,'aea357e42f53e027b18ab012a5cb9fb554bd3d0f2ab2a641a7eaa27b6632c656',21,'1FOucG_ROrsBiVL3XSLxVQSsSEzUJH0UK'),
]

def digest(data): return hashlib.sha256(data).hexdigest()
def gitblob(data): return hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
def save(path,value): path.write_text(json.dumps(value,indent=2,sort_keys=True)+'\n')

def decrypt(private, envelope, run_id):
    if envelope['run_id'] != run_id: raise ValueError('wrong run binding')
    parts=envelope['chunks']
    if not 1 <= len(parts) <= 32: raise ValueError('invalid chunk count')
    result=[]
    for i,part in enumerate(parts):
        raw=base64.b64decode(part,validate=True)
        if len(raw)!=512: raise ValueError('invalid RSA4096 ciphertext length')
        label=f'vigilode-import:{run_id}:{i}:{len(parts)}'.encode().hex()
        p=subprocess.run(['openssl','pkeyutl','-decrypt','-inkey',str(private),
            '-pkeyopt','rsa_padding_mode:oaep','-pkeyopt','rsa_oaep_md:sha256',
            '-pkeyopt','rsa_mgf1_md:sha256','-pkeyopt','rsa_oaep_label:'+label],
            input=raw,capture_output=True)
        if p.returncode: raise ValueError('encrypted request could not be authenticated')
        result.append(p.stdout)
    message=b''.join(result)
    if len(message)>12000 or digest(message)!=envelope['plaintext_sha256']:
        raise ValueError('message binding failed')
    data=json.loads(message)
    if data['run_id']!=run_id or set(data['files'])!={r[1]for r in EXPECTED}:
        raise ValueError('unexpected archive request')
    return data['files']

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self,req,fp,code,msg,headers,newurl):
        raise ValueError('download redirect not permitted')

def download(url,size):
    parsed=urllib.parse.urlsplit(url)
    if parsed.scheme!='https' or not parsed.hostname or not parsed.hostname.endswith('.oaiusercontent.com') or parsed.username or parsed.password:
        raise ValueError('download host outside authenticated file transport')
    with urllib.request.build_opener(NoRedirect).open(url,timeout=45)as response:
        data=response.read(size+1)
    if len(data)!=size: raise ValueError('download size mismatch')
    return data

def inspect_archive(alias,name,data):
    result=[];seen=set()
    with zipfile.ZipFile(io.BytesIO(data))as z:
        if z.testzip()is not None: raise ValueError('archive CRC failure')
        for item in z.infolist():
            if item.is_dir(): continue
            path=PurePosixPath(item.filename)
            mode=item.external_attr>>16
            if path.is_absolute() or '..'in path.parts or '\\'in item.filename or stat.S_ISLNK(mode):
                raise ValueError('unsafe archive member')
            dest=alias+'/'+str(path)
            if dest in seen or item.file_size>2*1024*1024: raise ValueError('duplicate/oversize member')
            seen.add(dest);b=z.read(item)
            result.append((dest,b,{'path':dest,'archive_name':name,'member':item.filename,
                'bytes':len(b),'sha256':digest(b),'git_blob_sha1':gitblob(b)}))
    return result

def import_data(blobs,receipt_dir):
    entries=[];archive_rows=[]
    for alias,name,size,sha,n,drive_id in EXPECTED:
        data=blobs[name]
        if len(data)!=size or digest(data)!=sha: raise ValueError('original ZIP identity mismatch: '+name)
        extracted=inspect_archive(alias,name,data)
        if len(extracted)!=n: raise ValueError('member count mismatch')
        entries.extend(extracted)
        archive_rows.append({'alias':alias,'name':name,'bytes':size,'sha256':sha,
            'git_blob_sha1':gitblob(data),'member_count':n,'drive_id':drive_id,
            'drive_url':f'https://drive.google.com/file/d/{drive_id}/view'})
    records=sorted((x[2]for x in entries),key=lambda x:x['path'])
    material=''.join(f"{r['path']}\0{r['bytes']}\0{r['sha256']}\n"for r in records).encode()
    if len(records)!=72 or digest(material)!=MEMBER_INDEX:
        raise ValueError('local-to-remote 72-member index does not match')
    dest=NODE/'original_sources';archives=NODE/'original_archives'
    writes=[(dest/p,b)for p,b,_ in entries]+[(archives/name,b)for name,b in blobs.items()]
    # Conflict check precedes all writes. Never replace a changed historical file.
    for p,b in writes:
        if p.is_symlink() or (p.exists()and p.read_bytes()!=b):
            raise ValueError('archive destination conflict: '+str(p))
    for p,b in writes:
        p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b)
        if p.read_bytes()!=b: raise ValueError('local writeback mismatch')
    manifest={'schema':'vigilode-published-originals-v1','archives':archive_rows,'members':records,
        'member_index_sha256':MEMBER_INDEX,'archive_files':4,'member_files':72,
        'member_bytes':sum(r['bytes']for r in records),
        'history_policy':'preserve bytes and historical failures; later reports control interpretation',
        'drive_sharing':'unchanged; backup files private','new_scientific_execution':False}
    save(NODE/'ORIGINALS_MANIFEST.json',manifest)
    receipt={'status':'PASS_SOURCE_COMPLETE_IMPORT','archive_files':4,'member_files':72,
        'member_index_sha256':MEMBER_INDEX,'member_bytes':manifest['member_bytes'],
        'zip_bytes':sum(r['bytes']for r in archive_rows),
        'archive_and_member_hashes_verified':76,'signed_urls_published':False,
        'private_key_published':False,'new_benchmark_runs':0,'solver_sources_modified':False}
    save(receipt_dir/'IMPORT_RECEIPT.json',receipt)
    return receipt

def main():
    private=Path(sys.argv[1]);envelope_path=Path(sys.argv[2]);receipt=Path(sys.argv[3]);rid=sys.argv[4]
    receipt.mkdir(parents=True,exist_ok=True)
    try:
        request=decrypt(private,json.loads(envelope_path.read_text()),rid)
        blobs={name:download(request[name],size)for _,name,size,_,_,_ in EXPECTED}
        print(json.dumps(import_data(blobs,receipt)))
    except Exception as e:
        # Download exceptions can contain a signed URL. Deliberately do not print
        # exception text/traceback; the type and fail-closed status are sufficient.
        save(receipt/'IMPORT_FAILURE.json',{'status':'IMPORT_FAILED','exception_type':type(e).__name__,
            'credential_output_redacted':True,'new_scientific_execution':False})
        print('IMPORT_FAILED:'+type(e).__name__)
        sys.exit(1)
    finally:
        private.unlink(missing_ok=True)
if __name__=='__main__':main()
