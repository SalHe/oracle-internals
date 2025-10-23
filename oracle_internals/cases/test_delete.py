from oracle_internals.cases.test_base import CaseBase


class TestDeleteTwoRow(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t1 (c1 number)')
                connection.begin()
                cursor.execute(f'insert into t1 values (1)')
                cursor.execute(f'insert into t1 values (2)')
                connection.commit()

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                cursor.execute(f'delete t1 where c1 = 1')
                cursor.execute(f'delete t1 where c1 = 2')
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')
