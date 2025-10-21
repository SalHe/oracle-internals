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


class TestInsertIntoVarchar(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t1 (c1 varchar(10))')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                for i in range(11):
                    s = 'A'*i
                    cursor.execute(f"insert into t1 values ('{s}')")
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')


class TestInsertIntoVarcharTwo(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute(
                    'create table t1 (c1 varchar(10), c2 varchar(10))')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                for i in range(11):
                    s = 'A'*i
                    s2 = 'A'*(10 - i)
                    cursor.execute(f"insert into t1 values ('{s}', '{s2}')")
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')


class TestInsertIntoVarcharThree(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute(
                    'create table t1 (c1 varchar(10), c2 varchar(10), c3 varchar(10))')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                for i in range(11):
                    s = 'A'*i
                    s2 = 'A'*(10 - i)
                    cursor.execute(
                        f"insert into t1 values ('{s}', '{s2}', '{s}')")
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')


class TestInsertIntoVarcharFour(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute(
                    'create table t1 (c1 varchar(10), c2 varchar(10), c3 varchar(10), c4 varchar(10))')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                for i in range(11):
                    s = 'A'*i
                    s2 = 'A'*(10 - i)
                    cursor.execute(
                        f"insert into t1 values ('{s}', '{s2}', '{s}', '{s2}')")
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')
