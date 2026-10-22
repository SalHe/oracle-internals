from oracle_internals.cases.test_base import CaseBase


class TestOverwriteRowMigration(CaseBase):
    """UPDATE that makes a row too large for its block, forcing row migration.

    Oracle emits three changes for the migrated row:

    - 11.2 (IRP): insert the row piece into the new block;
    - 11.4 (LKR): lock the row piece;
    - 11.6 (ORP): overwrite the old head piece in place with the forwarding
      address (`nrid`), leaving `cc: 0`.

    Direct-path inserts (`append`/`append_values`) and `CREATE TABLE AS SELECT`
    only produce 11.2/11.3/11.5 and never 11.6.
    """

    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute("""
                    create table t_orp_mig (
                        id     number,
                        v      varchar2(4000),
                        filler char(200)
                    ) pctfree 0
                """)
                cursor.execute("""
                    insert into t_orp_mig (id, v, filler)
                    select level, rpad('x', 700, 'x'), rpad('f', 200, 'f')
                    from dual connect by level <= 8
                """)
                connection.commit()

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute(
                    "update t_orp_mig set v = rpad('y', 3000, 'y') where id = 1")
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t_orp_mig')
