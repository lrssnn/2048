from MinimaxAgent import MinimaxAgent
from game import Game, Direction
from time import clock
from numpy import set_printoptions


def highestScore(agent):
    return agent.game.state.max()

# This prevents numpy arrays from printing in horrible scientific notation
set_printoptions(suppress=True)

result = ""

# Params
depth = 2
dynamic = False

game = Game(size=4)
agent = MinimaxAgent(game, depth, dynamicDepth=dynamic)

moves = 0
start = clock()
while not agent.game.over or len(agent.game.get_available_cells()) > 0:
    moves += 1
    agent.moveOnce()
    print game.state, "\n"
end = clock()

print game.state
time = end - start
result += "Depth: {} | Dynamic: {} | Time: {:.1f} | Score: {:.0f} | Moves/sec: {:.2f} | Points/sec: {:.2f} | Highest Tile: {:.0f}\n" \
    .format(depth, dynamic, time, agent.game.score, moves / time, agent.game.score / time, highestScore(agent))
print result
