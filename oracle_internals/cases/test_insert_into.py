from oracle_internals.cases.test_base import CaseBase


class TestInsertIntoOneColumn(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t1 (c1 number)')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                cursor.execute('insert into t1 values (1)')
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')


class TestInsertIntoOneNull(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t1 (c1 number)')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                cursor.execute('insert into t1 values (null)')
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')


class TestInsertIntoTwoColumns(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t2 (c1 number, c2 number)')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                cursor.execute('insert into t2 values (1, 2)')
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t2')
