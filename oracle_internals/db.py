import io
import tarfile
import docker
import oracledb
from docker.models.containers import Container
from oracle_internals.config import CONFIG


def get_container() -> Container | None:
    docker_client = docker.from_env()
    existing = docker_client.containers.list(
        all=True,
        filters={"name": CONFIG['container']['name']},
    )
    if existing:
        assert len(existing) == 1
        existing = existing[0]
        return existing

    return None


def db_connection():
    return oracledb.connect(user="sys", password=CONFIG['container']['db-password'], dsn="localhost:1521/ORCLINTERNAL", mode=oracledb.SYSDBA)


def get_file_content(container_path: str) -> bytes:
    container = get_container()
    stream, _ = container.get_archive(container_path) # type: ignore
    fileobj = io.BytesIO()
    for chunk in stream:
        fileobj.write(chunk)
    fileobj.seek(0)
    with tarfile.open(fileobj=fileobj) as tar:
        member = tar.getmember(container_path.split('/')[-1])
        stream = tar.extractfile(member)
        assert stream is not None
        return stream.read()  # type: ignore
