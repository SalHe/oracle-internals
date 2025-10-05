import pytest


@pytest.fixture
def get_connection():
    from oracle_internals.db import db_connection
    return db_connection
