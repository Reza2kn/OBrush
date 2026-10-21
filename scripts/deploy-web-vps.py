#!/usr/bin/env python3
"""Deploy an official static build over existing SSH authentication.

Retains previous releases; never stores credentials or removes other sites/data.
Use --stage-only while the domain's DNS change is pending.
"""
import argparse
import hashlib
import io
import ipaddress
import json
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tarfile


REMOTE = r'''
import fcntl,hashlib,ipaddress,json,os,pathlib,re,shutil,socket,stat,subprocess,sys,tarfile,time
release_id,manifest_sha,stage_only,expected_ip,expected_bytes=sys.argv[1:]
if not re.fullmatch(r'[0-9a-f]{12}-[0-9a-f]{12}',release_id):
    raise SystemExit('Invalid release identifier')
ipaddress.IPv4Address(expected_ip)
expected_bytes=int(expected_bytes)
if expected_bytes<1 or expected_bytes>256*1024*1024:
    raise SystemExit('Invalid upload size')
base=pathlib.Path('/var/www/obrush')
required=expected_bytes+128*1024*1024
if shutil.disk_usage('/var/www').free < required:
    raise SystemExit('Deployment needs upload size plus 128 MiB free on /var/www; no data removed.')
lock=open('/run/lock/obrush-deploy.lock','w')
fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
if base.is_symlink():raise SystemExit('Refusing a symlinked OBrush base directory')
base.mkdir(exist_ok=True)
releases=base/'releases'
if releases.is_symlink():raise SystemExit('Refusing a symlinked release directory')
releases.mkdir(exist_ok=True)
stage=releases/('.upload-'+release_id)
if stage.exists():
    raise SystemExit('An incomplete upload exists; inspect it before retrying: '+str(stage))
stage.mkdir()
stage.chmod(0o755)
manifest=None;seen=set();total=0
try:
    with tarfile.open(fileobj=sys.stdin.buffer,mode='r|gz') as archive:
        for member in archive:
            name=member.name
            parts=pathlib.PurePosixPath(name).parts
            if (not member.isfile() or not parts or name.startswith('/') or '..' in parts
                    or name!=pathlib.PurePosixPath(name).as_posix() or name in seen):
                raise ValueError('Invalid upload member')
            seen.add(name);total+=member.size
            if member.size>64*1024*1024 or total>256*1024*1024 or len(seen)>10000:
                raise ValueError('Upload allocation limit')
            data=archive.extractfile(member).read()
            if name=='_deploy-manifest.json':
                if manifest is not None or len(seen)!=1 or hashlib.sha256(data).hexdigest()!=manifest_sha:
                    raise ValueError('Invalid deployment manifest')
                manifest=json.loads(data)
            else:
                if manifest is None or manifest['files'].get(name)!=hashlib.sha256(data).hexdigest():
                    raise ValueError('Unexpected or corrupt file: '+name)
            destination=stage/name
            destination.parent.mkdir(parents=True,exist_ok=True)
            for directory in (destination.parent,*destination.parent.parents):
                if directory==stage:break
                directory.chmod(0o755)
            destination.write_bytes(data);destination.chmod(0o644)
    if manifest is None or seen!=set(manifest['files'])|{'_deploy-manifest.json'} or total!=expected_bytes:
        raise ValueError('Incomplete upload')
    release=releases/release_id
    if release.is_symlink():raise ValueError('Existing release is a symlink')
    if release.exists():
        if not release.is_dir() or (release/'_deploy-manifest.json').read_bytes()!=(stage/'_deploy-manifest.json').read_bytes():
            raise ValueError('Existing release identity differs')
        present=set()
        for path in release.rglob('*'):
            if path.is_symlink():raise ValueError('Existing release contains a symlink')
            if path.is_file():present.add(path.relative_to(release).as_posix())
        if present!=seen:
            raise ValueError('Existing release file set differs from the verified upload')
        for name,digest in manifest['files'].items():
            if hashlib.sha256((release/name).read_bytes()).hexdigest()!=digest:
                raise ValueError('Existing release file differs from the verified upload: '+name)
        shutil.rmtree(stage)
    else:
        stage.rename(release)
    for directory in (base,releases,release,*((p for p in release.rglob('*') if p.is_dir()))):
        if stat.S_IMODE(directory.stat().st_mode)&0o005!=0o005:
            raise ValueError('Release directory is not readable/traversable by Caddy: '+str(directory))
except BaseException:
    # Keep an incomplete upload for inspection. Never delete unrelated files.
    raise
receipt={'release':str(release),'manifest_sha256':manifest_sha,'source_commit':manifest['source_commit'],'activated':False}
if stage_only=='1':
    print(json.dumps(receipt));sys.exit(0)
for domain in ('obrush.app','www.obrush.app'):
    addresses={row[4][0] for row in socket.getaddrinfo(domain,443,type=socket.SOCK_STREAM)}
    if addresses!={expected_ip}:
        raise SystemExit('Release staged, but DNS for '+domain+' is not exclusively '+expected_ip)
config=pathlib.Path('/etc/caddy/sites-enabled/obrush.caddy')
main_config=pathlib.Path('/etc/caddy/Caddyfile')
if not re.search(r'(?m)^\s*import\s+/etc/caddy/sites-enabled/\*\s*(?:#.*)?$',main_config.read_text()):
    raise SystemExit('Release staged; main Caddyfile does not import the managed site directory')
config_bytes=(release/'_obrush.caddy').read_bytes()
marker=b'# Managed by OBrush deployment.'
if not config_bytes.startswith(marker):
    raise SystemExit('Release staged; candidate Caddy site lacks its managed marker')
if config.parent.is_symlink() or not config.parent.is_dir() or config.is_symlink():
    raise SystemExit('Release staged; refusing an unexpected Caddy site path')
previous_config=config.read_bytes() if config.exists() else None
if previous_config is not None and not previous_config.startswith(marker):
    raise SystemExit('Release staged; refusing to overwrite an unmanaged Caddy site')
current=base/'current'
if current.exists() and not current.is_symlink():
    raise SystemExit('Release staged; current is not a managed symlink')
previous_target=os.readlink(current) if current.is_symlink() else None
if previous_target is not None:
    resolved=current.resolve(strict=True)
    if resolved.parent!=releases.resolve(strict=True) or not resolved.is_dir():
        raise SystemExit('Release staged; current points outside managed releases')
def switch(target):
    temporary=base/'current.next'
    if temporary.exists() or temporary.is_symlink():raise ValueError('Unexpected current.next path')
    temporary.symlink_to(target);temporary.replace(current)
def write_config(data):
    temporary=config.with_name('obrush.caddy.next')
    if temporary.exists() or temporary.is_symlink():raise ValueError('Unexpected Caddy staging path')
    with temporary.open('xb') as output:
        output.write(data);output.flush();os.fsync(output.fileno())
    temporary.chmod(0o644)
    temporary.replace(config)
def command(args):
    p=subprocess.run(args,text=True,capture_output=True)
    if p.returncode:raise RuntimeError(' '.join(args[:2])+' failed: '+p.stderr[-2000:])
reload_attempted=False
config_installed=False
current_switched=False
try:
    write_config(config_bytes)
    config_installed=True
    command(['caddy','validate','--config','/etc/caddy/Caddyfile','--adapter','caddyfile'])
    switch(release)
    current_switched=True
    reload_attempted=True
    command(['caddy','reload','--config','/etc/caddy/Caddyfile','--adapter','caddyfile'])
    receipt.update(activated=True,previous_target=previous_target,activated_utc=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()))
    receipt_path=base/'last-deployment.json'
    receipt_next=base/'last-deployment.json.next'
    if receipt_next.exists() or receipt_next.is_symlink():raise ValueError('Unexpected deployment receipt staging path')
    with receipt_next.open('x') as output:
        output.write(json.dumps(receipt,indent=2)+'\n');output.flush();os.fsync(output.fileno())
    receipt_next.replace(receipt_path)
except BaseException:
    if config_installed:
        if previous_config is None:config.unlink(missing_ok=True)
        else:write_config(previous_config)
    if current_switched:
        if previous_target is None:current.unlink(missing_ok=True)
        else:switch(previous_target)
    if reload_attempted:
        try:command(['caddy','reload','--config','/etc/caddy/Caddyfile','--adapter','caddyfile'])
        except BaseException as rollback_error:
            raise RuntimeError('Deployment failed and previous Caddy config could not be reloaded') from rollback_error
    raise
print(json.dumps(receipt))
'''


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', required=True, help='Existing SSH destination, e.g. root@host')
    parser.add_argument('--expected-ip', required=True, help='Expected public IPv4 address for both domain names')
    parser.add_argument('--stage-only', action='store_true')
    parser.add_argument('--web-root', type=Path, default=Path('web/dist'))
    args = parser.parse_args()
    if not re.fullmatch(r'[a-zA-Z0-9_.@:-]+', args.host) or args.host.startswith('-'):
        parser.error('Invalid SSH host')
    try:
        ipaddress.IPv4Address(args.expected_ip)
    except ipaddress.AddressValueError:
        parser.error('--expected-ip must be an IPv4 address')
    root = Path(__file__).resolve().parents[1]
    web = args.web_root.resolve()
    info = json.loads((web / 'build-info.json').read_text())
    commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    dirty = subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True).strip()
    if dirty or info.get('source_tree_state') != 'clean' or info.get('source_commit') != commit:
        parser.error('Build and clean checkout must have the same source commit; run scripts/build-web.sh')
    if info.get('cargo_lock_sha256') != sha((root / 'Cargo.lock').read_bytes()):
        parser.error('Cargo.lock differs from the official build')
    for key, name in [('portable_wasm_sha256', 'pkg/obrush_app_bg.wasm'),
                      ('simd_wasm_sha256', 'pkg-simd/obrush_app_bg.wasm'),
                      ('gpu_wasm_sha256', 'gpu-pkg/resident_bg.wasm')]:
        if info.get(key) != sha((web / name).read_bytes()):
            parser.error('Official build hash mismatch: ' + name)
    files = {}
    for file in sorted(web.rglob('*')):
        if file.is_symlink():
            parser.error('Browser bundle contains a symlink')
        if file.is_file():
            files[file.relative_to(web).as_posix()] = file.read_bytes()
    files['_obrush.caddy'] = (root / 'deploy/obrush.caddy').read_bytes()
    manifest = json.dumps({'schema_version': 1, 'source_commit': commit,
                           'files': {name: sha(data) for name, data in files.items()}},
                          sort_keys=True, separators=(',', ':')).encode()
    manifest_sha = sha(manifest)
    release_id = commit[:12] + '-' + manifest_sha[:12]
    upload_bytes = len(manifest) + sum(len(data) for data in files.values())
    if upload_bytes > 256 * 1024 * 1024:
        parser.error('Browser bundle exceeds the 256 MiB upload limit')
    remote_command = 'python3 -c ' + shlex.quote(REMOTE) + ' ' + ' '.join(map(shlex.quote, [
        release_id, manifest_sha, '1' if args.stage_only else '0', args.expected_ip,
        str(upload_bytes)]))
    process = subprocess.Popen(['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10',
                                args.host, remote_command], stdin=subprocess.PIPE)
    try:
        with tarfile.open(fileobj=process.stdin, mode='w|gz') as archive:
            for name, data in [('_deploy-manifest.json', manifest), *files.items()]:
                member = tarfile.TarInfo(name)
                member.size = len(data)
                member.mode = 0o644
                archive.addfile(member, io.BytesIO(data))
        process.stdin.close()
    except BrokenPipeError:
        pass
    return process.wait()


if __name__ == '__main__':
    sys.exit(main())
