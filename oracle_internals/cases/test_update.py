from oracle_internals.cases.test_base import CaseBase


class TestUpdate1(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t1 (c1 number, c2 number, c3 number)')
                connection.begin()
                cursor.execute(f'insert into t1 values (1, 2, 3)')
                cursor.execute(f'insert into t1 values (1, 2, 3)')
                cursor.execute(f'insert into t1 values (2, 4, 6)')
                cursor.execute(f'insert into t1 values (2, 4, 6)')
                cursor.execute(f'insert into t1 values (3, 2, 3)')
                cursor.execute(f'insert into t1 values (3, 2, 3)')
                connection.commit()

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                cursor.execute(f'update t1 set c1 = 100, c2 = 200 where c1 = 1')
                cursor.execute(f'update t1 set c1 = 200, c2 = 400 where c1 = 2')
                cursor.execute(f'update t1 set c1 = 100, c3 = 200 where c1 = 3')
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')


class TestUpdate2(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t1 (c1 number, c2 number)')
                connection.begin()
                cursor.execute(f'insert into t1 values (1, 2)')
                cursor.execute(f'insert into t1 values (1, 2)')
                cursor.execute(f'insert into t1 values (1, 2)')
                cursor.execute(f'insert into t1 values (1, 2)')
                cursor.execute(f'insert into t1 values (1, 2)')
                cursor.execute(f'insert into t1 values (1, 2)')
                cursor.execute(f'insert into t1 values (1, 2)')
                cursor.execute(f'insert into t1 values (2, 4)')
                cursor.execute(f'insert into t1 values (2, 4)')
                connection.commit()

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                cursor.execute(
                    f'update t1 set c1 = 100, c2 = 200 where c1 = 1')
                cursor.execute(
                    f'update t1 set c1 = 200, c2 = 400 where c1 = 2')
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')
