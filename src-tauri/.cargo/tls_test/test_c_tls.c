__declspec(thread) int tls_var = 42;

int main(void) {
    if (tls_var != 42) return 1;
    tls_var = 99;
    if (tls_var != 99) return 2;
    return 0;
}
