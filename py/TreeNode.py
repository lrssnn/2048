# One node in a search tree which can be used for minimax or expectimax search.

# Requires state to have a method possibleChildren() which returns a list of every
# state which could result from a legal move on that current state. This method must
# also return a list of probabilities from 0 to 1 of those states occuring, that is,
# this list should add to one.


class TreeNode:

    # mode is either "Maximiser", "Minimiser" or "Expecter"
    def __init__(self, mode, state, algorithm, probability=0):
        self.children = []
        self.mode = mode
        self.gameState = state
        self.probability = probability
        self.algorithm = algorithm

    def getValue(self):
        # Calculate the value of the node, based on what type we are
        # No switch statement in python? What?
        if self.mode == "Maximiser":
            return self.getMaximiserValue()
        elif self.mode == "Minimiser":
            return self.getMinimiserValue()
        else:
            return self.getExpectedValue()

    # algorithm is either "Minimax" or "Expectimax"
    def generateChildren(self, maxDepth):
        # Not entirely sure what the interface is going to be here
        if maxDepth == 0:
            # Stop Generating
            return

        possibleChildren, childProbabilities = self.gameState.possibleChildren()

        for i in range(len(possibleChildren)):
            state = possibleChildren[i]
            if self.algorithm == "Minimax":
                self.children.append(TreeNode(self.alternateMode(), state, self.algorithm))
            elif self.algorithm == "Expectimax":
                prob  = childProbabilities[i]
                self.children.append(TreeNode(self.alternateMode(), state, self.algorithm, prob))
        for node in self.children:
                node.generateChildren(maxDepth - 1)

    def getMaximiserValue(self):
        if len(self.children) == 0:
            return self.heuristicValue()

        maxValue = -float("inf")
        for node in self.children:
            maxValue = max(maxValue, node.getValue())
        return maxValue

    def getMinimiserValue(self):
        if len(self.children) == 0:
            return self.heuristicValue()

        minValue = float("inf")
        for node in self.children:
            minValue = min(minValue, node.getValue())
        return minValue

    def getExpectedValue(self):
        if len(self.children) == 0:
            return self.heuristicValue()

        value = 0
        for node in self.children:
            value += node.getValue() * node.probability
        return value

    def heuristicValue(self):
        return self.gameState.getHeuristicValue()

    def alternateMode(self):
        if self.algorithm == "Minimax":
            return "Maximiser" if self.mode == "Minimiser" else "Minimiser"
        else:
            return "Maximiser" if self.mode == "Expecter" else "Expecter"

    def toString(self, depth):
        result = "Name: " + self.gameState.toString() + " Type: " + self.mode
        result += ("Prob: " + str(self.probability)) if self.mode == "Expecter" else ""
        result += '\n'
        for node in self.children:
            result += (depth * '---+') + node.toString(depth + 1)
        return result
