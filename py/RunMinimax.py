from MinimaxAgent import MinimaxAgent
from game import Game, Direction
from time import clock
from numpy import set_printoptions, mean


def highestScore(agent):
    return agent.game.state.max()

# This prevents numpy arrays from printing in horrible scientific notation
set_printoptions(suppress=True)

result = ""

# Params
depth = 3
dynamic = False

# Arrays for result taking
times      = []
scores     = []
movesPer   = []
pointsPer  = []
twofivesix = []
fivetwelve = []
oneK       = []
twoK       = []
fourK      = []
eightK     = []
sixteenK   = []
thirtytwoK = []
sixtyfourK = []

for _ in range(100):
    game = Game(size=5)
    agent = MinimaxAgent(game, depth, dynamicDepth=dynamic)

    moves = 0
    start = clock()
    while not agent.game.over or len(agent.game.get_available_cells()) > 0:
        moves += 1
        agent.moveOnce()
        # print "\rMove ", moves,
    end = clock()

    # print game.state
    time = end - start

    times.append(time)
    scores.append(game.score)
    movesPer.append(moves/time)
    pointsPer.append(game.score / time)
    twofivesix.append(game.max_block >= 256)
    fivetwelve.append(game.max_block >= 512)
    oneK.append(game.max_block >= 1024)
    twoK.append(game.max_block >= 2048)
    fourK.append(game.max_block >= 4096)
    eightK.append(game.max_block >= 8192)
    sixteenK.append(game.max_block >= 16384)
    thirtytwoK.append(game.max_block >= 32768)
    sixtyfourK.append(game.max_block >= 65536)

    result = "Run {} | Depth: {}{} | Time: {} | Score: {:.1f} | Mv/s: {:.1f} | Pt/s: {:.1f} | 256: {:.1f}% | 512: {:.1f}% | 1k: {:.1f}% | 2k: {:.1f}% | 4k: {:.1f}% | 8k: {:.1f}% | 16k: {:.1f}% | 32k: {:.1f}% | 64k: {:.1f}% | Total test time: {:.2f}hr"\
        .format(_+1, depth,
                "D" if dynamic else " ",
                mean(times),
                mean(scores),
                mean(movesPer),
                mean(pointsPer),
                mean([1 if yes else 0 for yes in twofivesix]) * 100,
                mean([1 if yes else 0 for yes in fivetwelve]) * 100,
                mean([1 if yes else 0 for yes in oneK]) * 100,
                mean([1 if yes else 0 for yes in twoK]) * 100,
                mean([1 if yes else 0 for yes in fourK]) * 100,
                mean([1 if yes else 0 for yes in eightK]) * 100,
                mean([1 if yes else 0 for yes in sixteenK]) * 100,
                mean([1 if yes else 0 for yes in thirtytwoK]) * 100,
                mean([1 if yes else 0 for yes in sixtyfourK]) * 100,
                sum(times) / 3600)
    print "\r", result,
