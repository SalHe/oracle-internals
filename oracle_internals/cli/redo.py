import os
import uuid
import io
import tarfile
import tempfile
from oracle_internals.cli.decorators import within_container
from oracle_internals.db import db_connection, must_get_container


def get_redo_path(redo):
    if os.path.isabs(redo) == False:
        redo = os.path.join(
            '/opt/oracle/oradata/ORCLINTERNAL/archive_logs/', redo)
    return redo


@within_container
def list_redo_files(args):
    container = must_get_container()
    path = '/opt/oracle/oradata/ORCLINTERNAL/archive_logs/'
    result = container.exec_run(['ls', '-lh', path])
    print(result.output.decode())


@within_container
def copy_redo(args):
    container = must_get_container()
    src = get_redo_path(args.src)
    dst = args.dst
    if os.path.isdir(dst) or dst.endswith(os.path.sep):
        dst = os.path.join(dst, os.path.basename(src))

    stream, _ = container.get_archive(src)
    import tarfile
    import io
    fileobj = io.BytesIO()
    for chunk in stream:
        fileobj.write(chunk)
    fileobj.seek(0)
    with tarfile.open(fileobj=fileobj) as tar:
        member = tar.getmembers()[0]
        with open(dst, 'wb') as f:
            f.write(tar.extractfile(member).read())  # type: ignore
    print(f"已复制 {src} 到 {dst}")


def _dump_one(redo_file):
    with db_connection() as connection:
        with connection.cursor() as cursor:
            sql = "select value from v$diag_info where name='Default Trace File'"
            cursor.execute(sql)

            diag_file = cursor.fetchone()
            assert diag_file is not None
            diag_file = diag_file[0]
            must_get_container().exec_run(  # type: ignore
                f'rm -f {diag_file}')

            cursor.execute(
                f"ALTER SYSTEM DUMP LOGFILE '{redo_file}'")

    with tempfile.NamedTemporaryFile() as f:
        from oracle_internals.db import download_file_from_container
        download_file_from_container(diag_file, f.name)
        with open(f.name) as f2:
            while True:
                line = f2.readline()
                if not line:
                    break
                print(line, end='')


@within_container
def dump_redo(args):
    container = must_get_container()
    if args.local_file:
        temp_name = f"/opt/oracle/oradata/ORCLINTERNAL/archive_logs/{uuid.uuid4().hex}_{os.path.basename(args.redo_file)}"
        with open(args.redo_file, "rb") as f:
            data = f.read()
        tar_stream = io.BytesIO()
        with tarfile.open(fileobj=tar_stream, mode="w") as tar:
            info = tarfile.TarInfo(name=os.path.basename(temp_name))
            info.size = len(data)
            tar.addfile(info, io.BytesIO(data))
        tar_stream.seek(0)
        container.put_archive(os.path.dirname(temp_name),
                              tar_stream)
        redo_file = temp_name
    else:
        redo_file = get_redo_path(args.redo_file)

    _dump_one(redo_file)

    if args.local_file:
        container.exec_run(f'rm -f {temp_name}')  # type: ignore
