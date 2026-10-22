from oracle_internals.cases.test_base import CaseBase


class TestIndexInsertLeafRow(CaseBase):
    """Insert rows into an indexed table; each insert emits 10.2 (kdxlin)."""

    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t_idx (c1 number, c2 varchar2(10))')
                cursor.execute('create index idx_t_idx on t_idx (c1)')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                cursor.execute("insert into t_idx values (1, 'a')")
                cursor.execute("insert into t_idx values (2, 'b')")
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t_idx')
