#include <errno.h>
#include <poll.h>
#include <string.h>
#include <unistd.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <sys/eventfd.h>
#include "test.h"

#define TEST(c, ...) ((c) ? 1 : (t_error(#c" failed: " __VA_ARGS__),0))
#define TESTE(c) (errno=0, TEST(c, "errno = %s\n", strerror(errno)))

/*
 *          t0
 *           | (ew)
 *          e0
 *           | (lt)
 *          p0
 */
void epoll1(void)
{
    int efd;
    int pfd[2];
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (ew)
 *          e0
 *           | (et)
 *          s0
 */
void epoll2(void)
{
    int efd;
    int pfd[2];
    struct epoll_event e;

    TESTE(pipe(pfd) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
}


/*
 *           t0
 *            | (ew)
 *           e0
 *     (lt) /  \ (lt)
 *        p0    p2
 */
void epoll3(void)
{
    int efd;
    int pfd[4];
    struct epoll_event events[2];

    TESTE(pipe(&pfd[0]) == 0);
    TESTE(pipe(&pfd[2]) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    events[0].events = EPOLLIN;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], events) == 0);

    events[0].events = EPOLLIN;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[2], events) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);
    TESTE(write(pfd[3], "w", 1) == 1);

    TEST(epoll_wait(efd, events, 2, 0) == 2, "epoll_wait return value should be 2\n");
    TEST(epoll_wait(efd, events, 2, 0) == 2, "epoll_wait return value should be 2\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
    close(pfd[2]);
    close(pfd[3]);
}


/*
 *           t0
 *            | (ew)
 *           e0
 *     (et) /  \ (et)
 *        s0    s2
 */
void epoll4(void)
{
    int efd;
    int pfd[4];
    struct epoll_event events[2];

    TESTE(pipe(&pfd[0]) == 0);
    TESTE(pipe(&pfd[2]) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    events[0].events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], events) == 0);

    events[0].events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[2], events) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);
    TESTE(write(pfd[3], "w", 1) == 1);

    TEST(epoll_wait(efd, events, 2, 0) == 2, "epoll_wait return value should be 2\n");
    TEST(epoll_wait(efd, events, 2, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
    close(pfd[2]);
    close(pfd[3]);
}

/*
 *          t0
 *           | (p)
 *          e0
 *           | (lt)
 *          s0
 */
void epoll5(void)
{
    int efd;
    int pfd[2];
    struct pollfd poll_fd;
    struct epoll_event e;

    TESTE(pipe(&pfd[0]) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    e.events = EPOLLIN;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
}

/*
 *          t0
 *           | (p)
 *          e0
 *           | (et)
 *          s0
 */
void epoll6(void)
{
    int efd;
    int pfd[2];
    struct pollfd poll_fd;
    struct epoll_event e;

    TESTE(pipe(&pfd[0]) == 0);

    efd = epoll_create(1);
    TESTE(efd >= 0);

    e.events = EPOLLIN | EPOLLET;
    TESTE(epoll_ctl(efd, EPOLL_CTL_ADD, pfd[0], &e) == 0);

    TESTE(write(pfd[1], "w", 1) == 1);

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 1, "poll return value should be 1\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 1, "epoll_wait return value should be 1\n");

    poll_fd.fd = efd;
    poll_fd.events = POLLIN;
    TEST(poll(&poll_fd, 1, 0) == 0, "poll return value should be 0\n");
    TEST(epoll_wait(efd, &e, 1, 0) == 0, "epoll_wait return value should be 0\n");

    close(efd);
    close(pfd[0]);
    close(pfd[1]);
}

int main(void)
{
    epoll1();
    // epoll2();
    epoll3();
    // epoll4();
    epoll5();
    // epoll6();
}