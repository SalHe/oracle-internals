from oracle_internals.cases.base import Case


class InsertIntoCase(Case):
    def prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t1 (c1 number)')

    def run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                cursor.execute('insert into t1 values (1)')
                connection.commit()

    def cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')
